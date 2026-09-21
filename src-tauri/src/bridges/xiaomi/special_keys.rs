//! 对齐 Python `XiaomiSpecialKeyHook`：抑制遥控器原生气
//!
//! 仅在「刚收到同键 HID direct / ATVV 信号」时吞掉 Windows 翻译的原 VK。

use crate::bridges::xiaomi::key_mapping::{
    direct_signal_recent, on_uncorrelated_f5_down, should_suppress_voice_f5, EXTRA_INFO,
};
use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, AtomicPtr, AtomicU32, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// 统一抑制窗口：LL hook 收到候选原生 VK 后，等待 pre_arm mark 的最长时间。
/// 实测时按命中率调整（越大越稳、越卡键盘；语音键 F5 用的是 80ms）。
const SUPPRESS_WAIT_MS: u64 = 30;

/// 钩子线程：重新 SetWindowsHookEx，把自己挂到链头（最后安装 = 最先调用）。
#[cfg(target_os = "windows")]
const WM_BUMP_HOOK_FRONT: u32 = 0x8000 + 71; // WM_APP + 71

static RUNNING: AtomicBool = AtomicBool::new(false);
static HOOK_THREAD_ID: AtomicU32 = AtomicU32::new(0);
static HID_TAP_READY: AtomicBool = AtomicBool::new(false);
static HOOK_ENABLED: AtomicBool = AtomicBool::new(true);
/// 仅由当前拥有线程持有的 JoinHandle；stop 必须 join 后才能再 start。
static HOOK_JOIN: Mutex<Option<std::thread::JoinHandle<()>>> = Mutex::new(None);

/// 遥控器正在注入 Alt 开头的组合键（如 Alt+Space, Alt+S），
/// 由 key_mapping 在 key_chord 注入前设置、注入后清除。
/// 钩子检测到此标志时，对带 EXTRA_INFO 的 Alt/Space 等系统键也进行特殊处理：
/// 吞掉原始的 WM_SYSKEYDOWN（防止系统菜单），改用 WM_KEYDOWN 放行。
static ALT_CHORD_ACTIVE: AtomicBool = AtomicBool::new(false);

/// 通知钩子：即将注入 Alt 组合键（key_mapping 在 SendInput 之前调用）
pub fn arm_alt_chord() {
    ALT_CHORD_ACTIVE.store(true, Ordering::Release);
}

/// 通知钩子：Alt 组合键注入完毕（key_mapping 在 SendInput 之后调用）
pub fn disarm_alt_chord() {
    ALT_CHORD_ACTIVE.store(false, Ordering::Release);
}

fn alt_chord_active() -> bool {
    ALT_CHORD_ACTIVE.load(Ordering::Acquire)
}

#[cfg(target_os = "windows")]
static HOOK_PTR: AtomicPtr<c_void> = AtomicPtr::new(std::ptr::null_mut());

/// 统一候选键：down 时等待 pre_arm mark（≤SUPPRESS_WAIT_MS），命中则吞原生键。
/// 注入键带 EXTRA_INFO，由 hook 的 injected 检查直接放行，不进入本路径（不会误吞）。
/// mark 在 recent 窗口（300ms）内持续保留：固件一次按压的多个报告周期产生的
/// 多个原生 down 都会被吞，避免"吞一个漏一个"。
/// up 放行：吞掉的 down 对应的原生 up 是孤立 keyup，对正常应用无害。
fn wait_and_consume_vk(vk: u32, scan: u32) -> Option<&'static str> {
    let (names, window, label): (&'static [&'static str], Duration, &'static str) = match vk {
        0xAF => (&["volume_up"], Duration::from_millis(200), "volume_up"),
        0xAE => (&["volume_down"], Duration::from_millis(200), "volume_down"),
        0xAD => (&["volume_mute", "mute"], Duration::from_millis(200), "volume_mute"),
        0xA6 => (&["back"], Duration::from_millis(250), "back"),
        0x24 | 0xAC => (&["home"], Duration::from_millis(250), "home"),
        0x5D => (&["menu"], Duration::from_millis(250), "menu"),
        0x0D => (&["ok"], Duration::from_millis(200), "ok"),
        0x25 => (&["left", "dpad_left"], Duration::from_millis(300), "left"),
        0x27 => (&["right", "dpad_right"], Duration::from_millis(300), "right"),
        0x26 => (&["up", "dpad_up"], Duration::from_millis(300), "up"),
        0x28 => (&["down", "dpad_down"], Duration::from_millis(300), "down"),
        0xC0 if scan == 0x29 => (&["tv"], Duration::from_millis(250), "tv"),
        0x5F | 0xFF => (&["power"], Duration::from_millis(250), "power"),
        _ if scan == 0x5E => (&["power"], Duration::from_millis(250), "power"),
        _ => return None,
    };
    let deadline = Instant::now() + Duration::from_millis(SUPPRESS_WAIT_MS);
    loop {
        if names.iter().any(|&n| direct_signal_recent(n, window)) {
            return Some(label);
        }
        if Instant::now() >= deadline {
            break;
        }
        std::thread::sleep(Duration::from_millis(2));
    }
    None
}

/// HID Tap 已验证 IO（可捕获返回/音量）后置 true
pub fn set_hid_tap_ready(ready: bool) {
    HID_TAP_READY.store(ready, Ordering::Release);
    log::info!("XIAOMI SPECIAL KEY hid_tap_ready={ready}");
}

pub fn hid_tap_ready() -> bool {
    HID_TAP_READY.load(Ordering::Acquire)
}

pub fn set_hook_enabled(enabled: bool) {
    HOOK_ENABLED.store(enabled, Ordering::Release);
}

/// 语音/录入恢复：把本进程 LL 钩子顶到链头。
/// **先挂新钩再卸旧钩**（overlap）。返回 generation，供 settle 等待。
pub fn bump_hook_to_front() -> u64 {
    let gen = crate::bridges::xiaomi::hook_bump::next_generation();
    #[cfg(target_os = "windows")]
    {
        use windows::Win32::Foundation::{LPARAM, WPARAM};
        use windows::Win32::UI::WindowsAndMessaging::PostThreadMessageW;
        HOOK_ENABLED.store(true, Ordering::Release);
        let tid = HOOK_THREAD_ID.load(Ordering::Acquire);
        log::info!("[DEBUG-cap] bump_hook_to_front tid={tid} armed={} gen={gen}", is_hook_armed());
        if tid == 0 {
            start_special_key_hook();
            return gen;
        }
        unsafe {
            let _ = PostThreadMessageW(tid, WM_BUMP_HOOK_FRONT, WPARAM(0), LPARAM(0));
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = gen;
    }
    gen
}

/// 请求置顶并等待真正落地。禁止在 LL 回调线程调用。
pub fn bump_hook_to_front_and_settle(settle_ms: u64) -> crate::bridges::xiaomi::hook_bump::BumpOutcome {
    let gen = bump_hook_to_front();
    #[cfg(target_os = "windows")]
    {
        use windows::Win32::System::Threading::GetCurrentThreadId;
        let current_tid = unsafe { GetCurrentThreadId() };
        let hook_tid = HOOK_THREAD_ID.load(Ordering::Acquire);
        let out = crate::bridges::xiaomi::hook_bump::wait_for(gen, current_tid, hook_tid, settle_ms);
        log::info!("[DEBUG-cap] bump settle gen={gen} out={out:?}");
        return out;
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = settle_ms;
        return crate::bridges::xiaomi::hook_bump::BumpOutcome::NoHookThread;
    }
    #[allow(unreachable_code)]
    crate::bridges::xiaomi::hook_bump::BumpOutcome::NoHookThread
}

/// 诊断/录入：钩子线程是否在跑
pub fn is_hook_running() -> bool {
    RUNNING.load(Ordering::Acquire)
}

/// LL 钩子是否已 SetWindowsHookEx 成功（比 RUNNING 更准）
pub fn is_hook_armed() -> bool {
    #[cfg(target_os = "windows")]
    {
        !HOOK_PTR.load(Ordering::Acquire).is_null()
    }
    #[cfg(not(target_os = "windows"))]
    {
        false
    }
}

/// 录入开始：只保证钩子线程在跑。
pub fn ensure_hook_for_capture() {
    HOOK_ENABLED.store(true, Ordering::Release);
    start_special_key_hook();
}

/// 整线程重启：**必须先 join 旧线程**，禁止双线程抢 HOOK_PTR。
pub fn restart_special_key_hook() {
    log::info!("[DEBUG-cap] hook thread restart begin");
    stop_special_key_hook(); // 内部 join
    HOOK_ENABLED.store(true, Ordering::Release);
    start_special_key_hook();
    let deadline = Instant::now() + Duration::from_millis(800);
    while !is_hook_armed() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(10));
    }
    log::info!(
        "[DEBUG-cap] hook thread restart done running={} armed={}",
        is_hook_running(),
        is_hook_armed()
    );
}

pub fn start_special_key_hook() {
    log::info!(
        "[DEBUG-cap] hook start req enabled={} running={}",
        HOOK_ENABLED.load(Ordering::Acquire),
        RUNNING.load(Ordering::Acquire)
    );
    if !HOOK_ENABLED.load(Ordering::Acquire) {
        log::info!("XIAOMI SPECIAL KEY hook disabled by config");
        return;
    }
    // 已在跑：什么都不做（禁止为了“确保”而拆掉活钩子）
    if RUNNING.load(Ordering::Acquire) {
        return;
    }
    // 可能残留未 join 的句柄（线程已退出但 handle 未 take）— 先收尸再起
    {
        let stale = HOOK_JOIN.lock().unwrap().is_some();
        if stale {
            stop_and_join_hook_thread();
        }
    }
    if RUNNING.load(Ordering::Acquire) {
        return;
    }
    RUNNING.store(true, Ordering::Release);
    let spawned = std::thread::Builder::new()
        .name("xiaomi-special-keys".into())
        .spawn(|| {
            #[cfg(target_os = "windows")]
            hook_loop();
            RUNNING.store(false, Ordering::Release);
            HOOK_THREAD_ID.store(0, Ordering::Release);
        });
    match spawned {
        Ok(handle) => {
            *HOOK_JOIN.lock().unwrap() = Some(handle);
            log::info!("XIAOMI SPECIAL KEY hook starting");
        }
        Err(_) => {
            RUNNING.store(false, Ordering::Release);
            log::error!("XIAOMI SPECIAL KEY hook thread spawn failed");
        }
    }
}

fn stop_and_join_hook_thread() {
    let handle = {
        let mut g = HOOK_JOIN.lock().unwrap();
        g.take()
    };
    if let Some(h) = handle {
        // 先请求退出再 join
        request_hook_thread_quit();
        let _ = h.join();
        RUNNING.store(false, Ordering::Release);
        HOOK_THREAD_ID.store(0, Ordering::Release);
        log::info!("[DEBUG-cap] hook thread joined");
    } else if RUNNING.load(Ordering::Acquire) {
        request_hook_thread_quit();
        let deadline = Instant::now() + Duration::from_millis(500);
        while RUNNING.load(Ordering::Acquire) && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(5));
        }
    }
}

fn request_hook_thread_quit() {
    #[cfg(target_os = "windows")]
    {
        use windows::Win32::UI::WindowsAndMessaging::{PostThreadMessageW, WM_QUIT};
        let tid = HOOK_THREAD_ID.load(Ordering::Acquire);
        if tid != 0 {
            unsafe {
                let _ = PostThreadMessageW(tid, WM_QUIT, None, None);
            }
        }
    }
}

pub fn stop_special_key_hook() {
    log::info!(
        "[DEBUG-cap] hook stop req running={} tid={}",
        RUNNING.load(Ordering::Acquire),
        HOOK_THREAD_ID.load(Ordering::Acquire)
    );
    HID_TAP_READY.store(false, Ordering::Release);
    stop_and_join_hook_thread();
    log::info!("XIAOMI SPECIAL KEY hook stop requested");
}

#[cfg(target_os = "windows")]
fn load_hook() -> windows::Win32::UI::WindowsAndMessaging::HHOOK {
    use windows::Win32::UI::WindowsAndMessaging::HHOOK;
    HHOOK(HOOK_PTR.load(Ordering::Acquire))
}

#[cfg(target_os = "windows")]
fn store_hook(h: windows::Win32::UI::WindowsAndMessaging::HHOOK) {
    HOOK_PTR.store(h.0, Ordering::Release);
}

/// 判断 vk 是否属于 Alt 按下时会被 Windows 系统拦截的组合键成员。
/// 包括 Alt 本身 + Space / F4 / Tab / Esc / 字母键等可能被全局热键占用的键。
#[cfg(target_os = "windows")]
fn is_alt_system_key(vk: u32) -> bool {
    // Alt 修饰键本身
    matches!(
        vk,
        0x12 | 0xA4 | 0xA5 | // VK_MENU, VK_LMENU, VK_RMENU
        0x20 | // VK_SPACE → Alt+Space 系统菜单
        0x73 | // VK_F4   → Alt+F4 关闭窗口
        0x09 | // VK_TAB  → Alt+Tab 任务切换
        0x1B    // VK_ESCAPE → Alt+Esc 切换窗口
    ) || (0x41u32..=0x5A).contains(&vk) // A-Z 可能被注册为全局热键
       || (0x30u32..=0x39).contains(&vk) // 0-9 可能被注册为全局热键
}

#[cfg(target_os = "windows")]
fn hook_loop() {
    use windows::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
    use windows::Win32::System::Threading::GetCurrentThreadId;
    use windows::Win32::UI::WindowsAndMessaging::{
        CallNextHookEx, DispatchMessageW, GetMessageW, SetWindowsHookExW, TranslateMessage,
        UnhookWindowsHookEx, HHOOK, KBDLLHOOKSTRUCT, MSG, WH_KEYBOARD_LL,
    };

    unsafe extern "system" fn proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        let hook = load_hook();
        if code >= 0 {
            let info = &*(lparam.0 as *const KBDLLHOOKSTRUCT);
            let flags = info.flags.0;
            let vk = info.vkCode;
            let msg = wparam.0 as u32;
            let injected = info.dwExtraInfo == EXTRA_INFO || (flags & 0x10) != 0;

            // 录入态：只记命中（原子），禁止在 LL 回调里做文件日志（会超时被 Windows 卸钩）
            if crate::bridges::shared::shortcut_capture::is_swallow_active() {
                crate::bridges::shared::shortcut_capture::note_hook_proc_hit(vk);
            }

            // 快捷键录入：最优先吞掉全部物理键
            if crate::bridges::shared::shortcut_capture::try_swallow_capture_key(vk, msg, injected)
            {
                return LRESULT(1);
            }

            // Alt 和弦注入中：即使是注入键（带 EXTRA_INFO），
            // 也不能直接放行 Alt/Space 等系统键 —— 否则会触发系统菜单。
            // 这里走抑制路径，让调用方（key_mapping）通过 WM_KEYDOWN 路径
            // 单独投递按键，避免 WM_SYSKEYDOWN。
            if alt_chord_active()
                && injected
                && is_alt_system_key(vk)
            {
                log::info!("XIAOMI SPECIAL KEY alt_chord suppressed vk=0x{vk:02X}");
                return LRESULT(1);
            }

            if injected {
                return CallNextHookEx(hook, code, wparam, lparam);
            }

            let scan = info.scanCode;
            let down = msg == 0x0100 || msg == 0x0104;
            let up = msg == 0x0101 || msg == 0x0105;

            // 统一抑制：
            //   down：最多等待 SUPPRESS_WAIT_MS 让 pre_arm 的 mark 到达，命中则吞原生键；
            //         mark 在 recent 窗口内保留：固件一次按压的多个报告周期产生的多个原生
            //         down 都会命中（避免吞一个漏一个）。注入键带 EXTRA_INFO，被上面的
            //         injected 检查直接放行，不进入本路径 → 每按恰好一次注入。
            //         未命中放行（= 实体键盘，等待仅造成 ≤SUPPRESS_WAIT_MS 延迟）。
            //   up：放行（原生 up 是孤立事件，无害；防止误吞注入 up 造成卡键）。
            //   F5 语音键保留独立的 sticky 逻辑。
            if vk == 0x74 {
                if down || up {
                    if should_suppress_voice_f5(down, up) {
                        crate::bridges::xiaomi::key_mapping::on_firmware_voice_key(down);
                        log::info!("XIAOMI SPECIAL KEY voice_f5 original_suppressed vk=0x{vk:02X}");
                        return LRESULT(1);
                    } else if down {
                        on_uncorrelated_f5_down();
                    }
                }
            } else if down {
                if let Some(name) = wait_and_consume_vk(vk, scan) {
                    log::info!("XIAOMI SPECIAL KEY {name} original_suppressed vk=0x{vk:02X} wait={SUPPRESS_WAIT_MS}ms");
                    return LRESULT(1);
                }
            }
        }
        CallNextHookEx(hook, code, wparam, lparam)
    }

    unsafe {
        HOOK_THREAD_ID.store(GetCurrentThreadId(), Ordering::Release);
        let hook = match SetWindowsHookExW(WH_KEYBOARD_LL, Some(proc), None, 0) {
            Ok(h) => h,
            Err(e) => {
                log::error!("SetWindowsHookExW failed: {e}");
                return;
            }
        };
        store_hook(hook);
        log::info!("[DEBUG-cap] hook armed tid={}", GetCurrentThreadId());
        log::info!(
            "XIAOMI SPECIAL KEYS READY mapping=configurable \
             repeat=back,volume,direction suppress_original=device-correlated"
        );
        let mut msg = MSG::default();
        while RUNNING.load(Ordering::Acquire) {
            let ret = GetMessageW(&mut msg, None, 0, 0);
            if ret.0 == -1 || ret.0 == 0 {
                break;
            }
            if msg.message == WM_BUMP_HOOK_FRONT {
                // 先挂新钩再卸旧钩；退出路径只卸「本 loop 自己创建」的句柄
                let old = load_hook();
                match SetWindowsHookExW(WH_KEYBOARD_LL, Some(proc), None, 0) {
                    Ok(h) => {
                        store_hook(h);
                        if !old.is_invalid() && old.0 != h.0 {
                            let _ = UnhookWindowsHookEx(old);
                        }
                        log::info!("[DEBUG-cap] bump overlap ok");
                    }
                    Err(e) => {
                        log::error!(
                            "[DEBUG-cap] bump SetWindowsHookEx failed: {e}; keep old hook"
                        );
                        if old.is_invalid() {
                            store_hook(HHOOK(std::ptr::null_mut()));
                        }
                    }
                }
                crate::bridges::xiaomi::hook_bump::mark_handled();
                continue;
            }
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
        // 退出：只卸本线程当前仍拥有的句柄；若 HOOK_PTR 已被新线程接管则不碰
        let mine = load_hook();
        let null = HHOOK(std::ptr::null_mut());
        // CAS：仅当仍是自己的句柄时清空
        #[cfg(target_os = "windows")]
        {
            use std::sync::atomic::AtomicPtr;
            let _ = HOOK_PTR.compare_exchange(
                mine.0,
                std::ptr::null_mut(),
                Ordering::AcqRel,
                Ordering::Acquire,
            );
        }
        let _ = null;
        if !mine.is_invalid() {
            let _ = UnhookWindowsHookEx(mine);
        }
        log::info!("[DEBUG-cap] hook loop exit");
    }
}

/// 音量键原生事件是否应被吞掉（避免与 SendInput 注入叠成系统双格）。
///
/// 背景：遥控器固件会额外冒出 VK_VOLUME_*，与 Tap/SendInput 叠成双格。
///
/// - **不得**用 `tap_ready` 无条件吞：实体音量±/静音与固件 VK 在 LL 无法区分
///   （与 Home/Menu 同原则；否则软件运行期间物理键盘音量键全部失效）；
/// - `recent_signal`：200ms 内遥控器刚按下 → 吞固件残留（HID Tap 侧尽早
///   `mark_direct_signal`，缩小 LL 先到窗口）。
///
/// 注意：非音量键（方向/OK/返回等）不受此判定影响。
pub fn should_suppress_volume_native(vk: u16, _tap_ready: bool, recent_signal: bool) -> bool {
    let is_volume = matches!(vk, 0xAF | 0xAE | 0xAD); // VK_VOLUME_UP / VK_VOLUME_DOWN / VK_VOLUME_MUTE
    // 仅 recent：tap_ready 会误伤实体音量键（与固件 VK_VOLUME_* 无法在 LL 区分）
    is_volume && recent_signal
}

/// menu/home 原生事件是否应被吞掉。
///
/// 背景：遥控器固件会额外冒出 VK_HOME(0x24)/0xAC / VK_APPS(0x5D)，与 Tap 注入叠成双击。
///
/// - **不得**用 `tap_ready` 无条件吞：实体 Home/菜单与遥控器固件 VK 在 LL 无法区分；
/// - `recent_signal`：250ms 内遥控器刚按下 → 吞固件残留（与 back/tv/power 一致）。
pub fn should_suppress_native_menu_home(vk: u16, _tap_ready: bool, recent_signal: bool) -> bool {
    let is_menu_or_home = matches!(vk, 0x5D | 0x24 | 0xAC); // VK_APPS / VK_HOME / 0xAC
    // 仅 recent：tap_ready 会误伤实体 Home（与固件 VK_HOME 无法在 LL 区分）
    is_menu_or_home && recent_signal
}

#[cfg(test)]
mod tests {
    use super::should_suppress_volume_native;

    #[test]
    fn volume_not_suppressed_by_tap_ready_alone() {
        // 与 Home/Menu 同原则：Tap 就绪不能单独吞，否则实体音量±/静音失效
        assert!(!should_suppress_volume_native(0xAF, true, false));
        assert!(!should_suppress_volume_native(0xAE, true, false));
        assert!(!should_suppress_volume_native(0xAD, true, false));
    }

    #[test]
    fn volume_suppressed_on_recent_signal() {
        // 200ms 内有遥控器信号：吞固件残留（防双格）
        assert!(should_suppress_volume_native(0xAF, false, true));
        assert!(should_suppress_volume_native(0xAE, true, true));
        assert!(should_suppress_volume_native(0xAD, false, true));
    }

    #[test]
    fn volume_passthrough_when_neither_ready() {
        // 无近期信号：透传原生事件（物理键盘音量键必须可用）
        assert!(!should_suppress_volume_native(0xAF, false, false));
        assert!(!should_suppress_volume_native(0xAE, false, false));
        assert!(!should_suppress_volume_native(0xAD, false, false));
    }

    #[test]
    fn non_volume_keys_never_affected() {
        // 方向键 0x26/0x28、OK 0x0D、返回 0xA6 不受音量判定影响
        assert!(!should_suppress_volume_native(0x26, true, true));
        assert!(!should_suppress_volume_native(0x28, true, true));
        assert!(!should_suppress_volume_native(0x0D, true, false));
        assert!(!should_suppress_volume_native(0xA6, true, false));
    }

    // ---- menu/home：仅 recent（tap_ready 不得误伤实体 Home）----

    use super::should_suppress_native_menu_home;

    #[test]
    fn menu_home_not_suppressed_by_tap_ready_alone() {
        // 与 F5 同原则：Tap 就绪不能单独吞，否则实体 Home/菜单键失效
        assert!(!should_suppress_native_menu_home(0x5D, true, false));
        assert!(!should_suppress_native_menu_home(0x24, true, false));
        assert!(!should_suppress_native_menu_home(0xAC, true, false));
    }

    #[test]
    fn menu_home_suppressed_on_recent_signal() {
        assert!(should_suppress_native_menu_home(0x5D, false, true));
        assert!(should_suppress_native_menu_home(0x24, true, true));
        assert!(should_suppress_native_menu_home(0xAC, false, true));
    }

    #[test]
    fn menu_home_passthrough_when_neither_ready() {
        // 两者皆无：透传原生事件（物理键盘 Home / Menu 键必须可用）
        assert!(!should_suppress_native_menu_home(0x5D, false, false));
        assert!(!should_suppress_native_menu_home(0x24, false, false));
        assert!(!should_suppress_native_menu_home(0xAC, false, false));
    }

    #[test]
    fn non_menu_home_keys_never_affected() {
        // Space 0x20（home 的注入目标）、Back 0xA6、OK 0x0D 不受此判定影响
        assert!(!should_suppress_native_menu_home(0x20, true, true));
        assert!(!should_suppress_native_menu_home(0xA6, true, true));
        assert!(!should_suppress_native_menu_home(0x0D, true, false));
    }
}
