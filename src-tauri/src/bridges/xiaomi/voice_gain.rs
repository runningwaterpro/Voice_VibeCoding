//! 麦克风增益 (dB) 热更新：配置保存与 ATVV 音频路径共享同一 live 值。
//!
//! 范围与前端 `XiaomiSettings.vue` 的 `GAIN_MIN` / `GAIN_MAX` 保持一致。
//! 自动增益算法参考 WebRTC AGC2 adaptive-digital 模式：
//! - 目标电平 -18 dBFS（语音标准，接近绿区右侧）
//! - 攻击（信号变响 → 降增益）快，释放（信号变弱 → 升增益）慢
//! - 噪声门限以下冻结增益，防底噪放大
//! - 帧级限速，防泵浦效应

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

/// 与 UI 步进器一致
pub const GAIN_DB_MIN: f32 = -12.0;
pub const GAIN_DB_MAX: f32 = 30.0;
pub const GAIN_DB_DEFAULT: f32 = 10.0;

// ---- 自动增益参数（WebRTC AGC2 风格）----
/// 目标输入电平 dBFS（WebRTC 默认 -18，此处略高贴近绿区右缘）
const AGC_TARGET_DBFS: f32 = -18.0;
/// 噪声门限：低于此视为静音，冻结增益
const AGC_NOISE_GATE_DBFS: f32 = -40.0;
/// 攻击系数（信号响 → 快速降增益），~250ms 时间常数 @50fps
const AGC_ATTACK: f32 = 0.08;
/// 释放系数（信号弱 → 慢速升增益），~2s 时间常数 @50fps
const AGC_RELEASE: f32 = 0.01;
/// 单帧最大增益变化（dB），防泵浦
const AGC_MAX_STEP_DB: f32 = 0.5;
/// 输入电平 EWMA 平滑系数
const AGC_EWMA_ALPHA: f32 = 0.3;

static LIVE_GAIN_DB: AtomicU32 = AtomicU32::new(GAIN_DB_DEFAULT.to_bits());
static AUTO_ENABLED: AtomicBool = AtomicBool::new(false);
/// 上一帧平滑后的输入电平 dBFS
static PREV_INPUT_DB: AtomicU32 = AtomicU32::new((-60.0f32).to_bits());
/// 连续有声帧计数（VAD 迟滞：需连续 N 帧有声才开始调增益）
static SPEECH_FRAMES: AtomicU32 = AtomicU32::new(0);

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
        PREV_INPUT_DB.store((-60.0f32).to_bits(), Ordering::Release);
        SPEECH_FRAMES.store(0, Ordering::Release);
    }
    log::info!("voice gain auto: {on}");
}

pub fn auto_enabled() -> bool {
    AUTO_ENABLED.load(Ordering::Acquire)
}

/// 每帧调用：根据输入电平自动调整增益。仅在 auto 模式下生效。
/// `input_level` 是 voice_meter 算出的 0..1 RMS+峰值混合值。
pub fn auto_adjust(input_level: f32) {
    if !auto_enabled() {
        return;
    }
    let input_db = if input_level > 0.0 {
        20.0 * input_level.log10()
    } else {
        -90.0
    };

    // 噪声门限以下：视为静音，冻结增益
    if input_db < AGC_NOISE_GATE_DBFS {
        SPEECH_FRAMES.store(0, Ordering::Release);
        return;
    }

    // EWMA 平滑输入电平
    let prev = f32::from_bits(PREV_INPUT_DB.load(Ordering::Acquire));
    let smoothed = prev * (1.0 - AGC_EWMA_ALPHA) + input_db * AGC_EWMA_ALPHA;
    PREV_INPUT_DB.store(smoothed.to_bits(), Ordering::Release);

    // VAD 迟滞：需连续 3 帧超过门限才开始调整
    let n = SPEECH_FRAMES.fetch_add(1, Ordering::AcqRel);
    if n < 3 {
        return;
    }

    // 误差 → 增益步长（非对称 attack/release）
    // 关键：比较目标与「输出电平」(输入+增益)，而非仅输入。
    // 否则增益越高误差越偏正 → 正反馈锁死在高增益。
    let output_db = smoothed + gain_db();
    let err = AGC_TARGET_DBFS - output_db;
    let rate = if err < 0.0 {
        // 输出已超目标 → 降增益（attack，快）
        AGC_ATTACK
    } else {
        // 输出低于目标 → 升增益（release，慢）
        AGC_RELEASE
    };
    let step = (err * rate).clamp(-AGC_MAX_STEP_DB, AGC_MAX_STEP_DB);
    let current = gain_db();
    let new_gain = normalize_gain_db(current + step);
    if (new_gain - current).abs() > f32::EPSILON {
        set_gain_db(new_gain);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn auto_adjust_disabled_is_noop() {
        set_auto_enabled(false);
        set_gain_db(10.0);
        auto_adjust(0.5);
        assert_eq!(gain_db(), 10.0);
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
        assert!(after_quiet > 10.0, "phase1 quiet should raise gain, got {}", after_quiet);

        // 阶段2：大声输入 0.5 ≈ -6dBFS，输出 = -6+gain 远超目标，attack 应降增益
        for _ in 0..200 {
            auto_adjust(0.5);
        }
        let after_loud = gain_db();
        assert!(
            after_loud < after_quiet,
            "phase2 loud should DECREASE gain from {}, but got {}",
            after_quiet, after_loud
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
