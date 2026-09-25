//! 麦克风增益 (dB) 热更新：配置保存与 ATVV 音频路径共享同一 live 值。
//!
//! 范围与前端 `XiaomiSettings.vue` 的 `GAIN_MIN` / `GAIN_MAX` 保持一致。
//! 自动增益算法参考 WebRTC AGC2 adaptive-digital 模式：
//! - 目标电平 -18 dBFS（语音标准，接近绿区右侧）
//! - 攻击（信号变响 → 降增益）快，释放（信号变弱 → 升增益）慢
//! - 只有绝对静音门限以下冻结增益，-60..-40 dBFS 的远距离语音仍可慢速升增益
//! - 按真实时间限速，防泵浦效应

use parking_lot::Mutex;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::time::Instant;

/// 与 UI 步进器一致
pub const GAIN_DB_MIN: f32 = -12.0;
pub const GAIN_DB_MAX: f32 = 30.0;
pub const GAIN_DB_DEFAULT: f32 = 10.0;

// ---- 自动增益参数（WebRTC AGC2 风格）----
/// 目标输入电平 dBFS（WebRTC 默认 -18，此处略高贴近绿区右缘）
const AGC_TARGET_DBFS: f32 = -18.0;
/// 绝对静音门限：低于此不放大，避免把静音底噪推起来
const AGC_SILENCE_GATE_DBFS: f32 = -60.0;
/// 低于此仍可进入慢速升增益，但不再按普通语音速度处理
const AGC_SPEECH_BAND_DBFS: f32 = -40.0;
/// 信号变响时快速降增益的时间常数
const AGC_ATTACK_TAU_SECONDS: f32 = 0.12;
/// 普通偏轻输入的升增益时间常数
const AGC_RELEASE_TAU_SECONDS: f32 = 0.35;
/// 远距离弱输入的升增益时间常数
const AGC_QUIET_RELEASE_TAU_SECONDS: f32 = 0.25;
/// 单次更新的最大增益变化（dB），防止泵浦
const AGC_MAX_STEP_DB: f32 = 0.75;
/// 最短控制间隔，按 ATVV 常见 15ms 帧处理
const AGC_MIN_STEP_SECONDS: f32 = 0.015;
/// 输入电平 EWMA 平滑系数
const AGC_EWMA_ALPHA: f32 = 0.3;

struct AgcState {
    prev_input_db: f32,
    non_silent_frames: u32,
    last_adjust: Option<Instant>,
}

static AGC_STATE: Mutex<AgcState> = Mutex::new(AgcState {
    prev_input_db: -60.0,
    non_silent_frames: 0,
    last_adjust: None,
});
static LIVE_GAIN_DB: AtomicU32 = AtomicU32::new(GAIN_DB_DEFAULT.to_bits());
static AUTO_ENABLED: AtomicBool = AtomicBool::new(false);

/// 规范化增益 dB（NaN → 默认，超出范围 clamp）。
pub fn normalize_gain_db(gain_db: f32) -> f32 {
    if gain_db.is_nan() {
        GAIN_DB_DEFAULT
    } else {
        gain_db.clamp(GAIN_DB_MIN, GAIN_DB_MAX)
    }
}

/// 更新 live 增益（配置落盘成功后或会话建立时调用）。
pub fn set_gain_db(gain_db: f32) {
    let clamped = normalize_gain_db(gain_db);
    let prev = f32::from_bits(LIVE_GAIN_DB.load(Ordering::Acquire));
    LIVE_GAIN_DB.store(clamped.to_bits(), Ordering::Release);
    if (prev - clamped).abs() > f32::EPSILON {
        log::info!("voice gain live: {prev:.1} dB -> {clamped:.1} dB");
    }
}

/// 当前应用于 PCM postprocess 的增益 dB。
pub fn gain_db() -> f32 {
    f32::from_bits(LIVE_GAIN_DB.load(Ordering::Acquire))
}

/// 开关自动增益模式。
pub fn set_auto_enabled(on: bool) {
    AUTO_ENABLED.store(on, Ordering::Release);
    if on {
        // 重置内部状态，避免上次残留
        let mut state = AGC_STATE.lock();
        state.prev_input_db = -60.0;
        state.non_silent_frames = 0;
        state.last_adjust = None;
    }
    log::info!("voice gain auto: {on}");
}

/// 开始一次新的按压会话。自动增益从配置中的手动基准重新开始，
/// 不继承上一句话的 live 值或控制器状态。
pub fn begin_voice_gain_session(auto_enabled: bool, manual_gain_db: f32) {
    set_auto_enabled(auto_enabled);
    set_gain_db(manual_gain_db);
}

pub fn auto_enabled() -> bool {
    AUTO_ENABLED.load(Ordering::Acquire)
}

/// 每帧调用：根据输入电平自动调整增益。仅在 auto 模式下生效。
/// `input_level` 是 voice_meter 算出的 0..1 RMS+峰值混合值。
pub fn auto_adjust(input_level: f32) {
    auto_adjust_at(input_level, Instant::now());
}

fn auto_adjust_at(input_level: f32, now: Instant) {
    if !auto_enabled() {
        return;
    }
    let input_level = if input_level.is_finite() {
        input_level.clamp(0.0, 1.0)
    } else {
        0.0
    };
    let input_db = if input_level > 0.0 {
        20.0 * input_level.log10()
    } else {
        -90.0
    };
    let mut state = AGC_STATE.lock();

    // 只有绝对静音才冻结；-60..-40 dBFS 的远距离语音仍可慢速升增益。
    if input_db < AGC_SILENCE_GATE_DBFS {
        state.non_silent_frames = 0;
        state.prev_input_db = input_db;
        state.last_adjust = Some(now);
        return;
    }

    // EWMA 平滑输入电平
    let smoothed = state.prev_input_db * (1.0 - AGC_EWMA_ALPHA) + input_db * AGC_EWMA_ALPHA;
    state.prev_input_db = smoothed;
    state.non_silent_frames = state.non_silent_frames.saturating_add(1);

    // 控制器按真实经过时间推进，而不是假设每个音频帧都是固定长度。
    // 测试或异常帧间隔极短时仍使用一个 15ms 的最小步长。
    let elapsed = state
        .last_adjust
        .take()
        .map(|last| now.duration_since(last).as_secs_f32())
        .unwrap_or(AGC_MIN_STEP_SECONDS)
        .max(AGC_MIN_STEP_SECONDS);
    state.last_adjust = Some(now);

    // 非静音迟滞：需连续 3 帧超过静音门限才开始调整。
    if state.non_silent_frames < 3 {
        return;
    }

    // 误差 → 增益步长。比较目标与输出电平（输入+增益），避免正反馈锁死。
    let output_db = smoothed + gain_db();
    let err = AGC_TARGET_DBFS - output_db;
    let quiet = input_db < AGC_SPEECH_BAND_DBFS;
    let tau = if err < 0.0 {
        AGC_ATTACK_TAU_SECONDS
    } else if quiet {
        AGC_QUIET_RELEASE_TAU_SECONDS
    } else {
        AGC_RELEASE_TAU_SECONDS
    };
    let fraction = 1.0 - (-elapsed / tau).exp();
    let step = (err * fraction).clamp(-AGC_MAX_STEP_DB, AGC_MAX_STEP_DB);
    let current = gain_db();
    let new_gain = normalize_gain_db(current + step);
    if (new_gain - current).abs() > f32::EPSILON {
        set_gain_db(new_gain);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn gain_clamps_to_ui_range() {
        set_gain_db(99.0);
        assert_eq!(gain_db(), GAIN_DB_MAX);
        set_gain_db(-99.0);
        assert_eq!(gain_db(), GAIN_DB_MIN);
        set_gain_db(14.5);
        assert!((gain_db() - 14.5).abs() < f32::EPSILON);
        set_gain_db(f32::NAN);
        assert_eq!(gain_db(), GAIN_DB_DEFAULT);
        assert_eq!(normalize_gain_db(f32::INFINITY), GAIN_DB_MAX);
        assert_eq!(normalize_gain_db(f32::NEG_INFINITY), GAIN_DB_MIN);
        set_gain_db(GAIN_DB_DEFAULT);
    }

    #[test]
    fn begin_voice_gain_session_resets_auto_gain_to_manual_baseline() {
        set_auto_enabled(true);
        set_gain_db(10.0);
        let quiet_level = 10f32.powf(-50.0 / 20.0);
        for _ in 0..20 {
            auto_adjust(quiet_level);
        }
        assert!(gain_db() > 10.0);
        begin_voice_gain_session(true, 10.0);
        assert_eq!(gain_db(), 10.0);
        set_auto_enabled(false);
    }

    #[test]
    fn auto_adjust_disabled_is_noop() {
        set_auto_enabled(false);
        set_gain_db(10.0);
        auto_adjust(0.5);
        assert_eq!(gain_db(), 10.0);
    }

    #[test]
    fn quiet_burst_reaches_useful_gain_within_300ms() {
        set_auto_enabled(true);
        set_gain_db(10.0);
        let quiet_level = 10f32.powf(-50.0 / 20.0);
        // 20 个典型 15ms ATVV 帧，模拟约 300ms 的远距离语音起音。
        let start = Instant::now();
        for frame in 0..20 {
            auto_adjust_at(quiet_level, start + Duration::from_millis((frame + 1) * 15));
        }
        assert!(
            gain_db() >= 12.0,
            "far speech should raise gain within about 300ms, got {}",
            gain_db()
        );
        set_auto_enabled(false);
    }

    #[test]
    fn auto_adjust_converges_toward_target() {
        set_auto_enabled(true);
        set_gain_db(10.0);
        // 0.01 ≈ -40 dBFS，接近门限但有声
        // 目标 -18，输入 -40 → err = +22 → release 升增益
        for _ in 0..200 {
            auto_adjust(0.01);
        }
        assert!(gain_db() > 10.0, "gain should increase, got {}", gain_db());
        set_auto_enabled(false);
    }

    #[test]
    fn auto_adjust_freezes_on_silence() {
        set_auto_enabled(true);
        set_gain_db(10.0);
        // 0.0001 ≈ -80 dBFS，低于噪声门限
        for _ in 0..50 {
            auto_adjust(0.0001);
        }
        assert_eq!(gain_db(), 10.0, "gain should not change on silence");
        set_auto_enabled(false);
    }

    #[test]
    fn auto_adjust_reduces_gain_when_loud() {
        set_auto_enabled(true);
        set_gain_db(0.0);
        // 0.5 ≈ -6 dBFS，高于目标 -18 → attack 降增益
        for _ in 0..200 {
            auto_adjust(0.5);
        }
        assert!(gain_db() < 0.0, "gain should decrease, got {}", gain_db());
        set_auto_enabled(false);
    }

    /// 复现用户症状：先变小，再变大，之后不再变小。
    /// 用户观察：开启 auto → gain 先降 → 后升 → 之后不再降。
    #[test]
    fn auto_adjust_should_decrease_again_after_increase() {
        set_auto_enabled(true);
        set_gain_db(10.0);

        // 阶段1：安静输入（0.01 ≈ -40dBFS），release 升增益
        for _ in 0..100 {
            auto_adjust(0.01);
        }
        let after_quiet = gain_db();
        assert!(
            after_quiet > 10.0,
            "phase1 quiet should raise gain, got {}",
            after_quiet
        );

        // 阶段2：大声输入 0.5 ≈ -6dBFS，输出 = -6+gain 远超目标，attack 应降增益
        for _ in 0..200 {
            auto_adjust(0.5);
        }
        let after_loud = gain_db();
        assert!(
            after_loud < after_quiet,
            "phase2 loud should DECREASE gain from {}, but got {}",
            after_quiet,
            after_loud
        );
        set_auto_enabled(false);
    }

    /// 核心 bug 回归：高增益 + 中等输入，输出已爆但旧算法误判为安静。
    /// 旧算法 err = target - input（漏了 gain）→ 正反馈锁死高增益。
    #[test]
    fn auto_adjust_high_gain_moderate_input_decreases() {
        set_auto_enabled(true);
        // 增益拉到最大
        set_gain_db(GAIN_DB_MAX);
        // 输入 -20 dBFS ≈ 0.1，输出 = -20+30 = +10 dBFS（严重爆音）
        // 正确算法：err = -18 - 10 = -28 → attack 降增益
        // 旧算法：err = -18 - (-20) = +2 → release 升增益（锁死）
        for _ in 0..200 {
            auto_adjust(0.1);
        }
        assert!(
            gain_db() < GAIN_DB_MAX,
            "gain should drop from max, but stuck at {}",
            gain_db()
        );
        set_auto_enabled(false);
    }
}
