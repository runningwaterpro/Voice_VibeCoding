//! 主窗口生命周期：托盘回收 / 按需重建 / reload → recreate 自救。
//!
//! 托盘常驻时整界面（6 个 WebView2 进程，约 150MB）是纯浪费，所以最小化后
//! 静置 [`UI_RECYCLE_DELAY`] 就销毁窗口；点托盘、二次启动、托盘菜单再按需
//! 重建。这和 `hide()` 不是一回事：隐藏后由 Windows 自行决定何时回收
//! WebView2，时机和状态都不可控（白屏/黑屏的来源）；这里是主动 `destroy()`
//! 后重建，路径确定，并且复用原本白屏自救的 create 逻辑。

use crate::webview_guard::{self, HealthAction};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

const MAIN_LABEL: &str = "main";

/// 托盘静置多久后销毁整个 WebView2 界面。
///
/// 延迟内“最小化又马上打开”仍是瞬时恢复；超过后点托盘要重建窗口，
/// 约 0.5~1 秒。收益：托盘常驻内存从约 160MB 降到约 10MB。
const UI_RECYCLE_DELAY: Duration = Duration::from_secs(30);

/// 下载/更新把回收推迟到下一轮的最大次数；超过就放弃本轮回收。
const UI_RECYCLE_MAX_DEFER: u32 = 20;

/// 重建后等前端自己显示的最长时间；超时兜底显示，避免前端挂掉时一片空白。
const UI_FIRST_SHOW_FALLBACK: Duration = Duration::from_secs(3);

/// 每次最小化/还原都递增，用来作废已经排队的回收任务。
static RECYCLE_SEQ: AtomicU64 = AtomicU64::new(0);

/// 托盘“一键修复”在界面已回收时的待办标记；由前端挂载后主动取走。
static PENDING_TRAY_REPAIR: AtomicBool = AtomicBool::new(false);

/// 最近一次广播给前端的可见性；`Resized` 会连续触发，只在真正变化时发事件。
static LAST_UI_VISIBLE: AtomicBool = AtomicBool::new(true);

/// [`try_reload_main`] 的结果：真的 reload 了，还是窗口已被回收、改成重建。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RefreshOutcome {
    Reloaded,
    Rebuilt,
}

/// 把"窗口能否被用户看见"同步到 [`crate::ui_state`]，并在变化时通知前端。
///
/// 这是可见性的唯一判据：最小化到托盘时 `document.hidden` 不可靠，实测前端
/// 轮询仍在跑。前端收到事件后暂停/恢复轮询和电平刷新。
fn sync_ui_visibility(app: &AppHandle, window: &WebviewWindow) {
    let visible = !window.is_minimized().unwrap_or(false) && window.is_visible().unwrap_or(false);
    crate::ui_state::set_window_visible(visible);
    if LAST_UI_VISIBLE.swap(visible, Ordering::SeqCst) != visible {
        let _ = app.emit("ui-visibility", visible);
        log::info!("WINDOW: ui visible={visible}");
    }
}

/// 启动时是否进入托盘（设置「启动后最小化到托盘」或 `--minimized`）
static BOOT_TO_TRAY: AtomicBool = AtomicBool::new(false);

pub fn set_boot_to_tray(v: bool) {
    BOOT_TO_TRAY.store(v, Ordering::SeqCst);
}

pub fn boot_to_tray() -> bool {
    BOOT_TO_TRAY.load(Ordering::SeqCst)
}

/// 下载/更新进行中时不要回收界面：前端监听一断，用户就看不到结果。
fn ui_busy() -> bool {
    crate::app_update::download_in_progress() || crate::file_download::downloads_active()
}

/// 作废已经排队的回收任务（窗口重新可见时调用）。
fn cancel_ui_recycle() {
    RECYCLE_SEQ.fetch_add(1, Ordering::SeqCst);
}

/// 最小化到托盘后排队销毁窗口，释放 WebView2 的 6 个进程。
///
/// 只销毁、不重建；点托盘时由 [`restore_main_window`] 按需重建。
/// `attempt` 是已因下载而推迟的轮数，用来给重试设上限。
fn schedule_ui_recycle(app: &AppHandle, attempt: u32) {
    let seq = RECYCLE_SEQ.fetch_add(1, Ordering::SeqCst) + 1;
    let app = app.clone();
    let spawned = std::thread::Builder::new()
        .name("ui-recycle".into())
        .spawn(move || {
            std::thread::sleep(UI_RECYCLE_DELAY);
            if RECYCLE_SEQ.load(Ordering::SeqCst) != seq {
                return;
            }
            if crate::ui_state::ui_visible() {
                return;
            }
            let app_main = app.clone();
            if let Err(e) = app.run_on_main_thread(move || {
                // 和托盘点击在同一线程上判定，避免“刚点开又被销毁”的竞态。
                if RECYCLE_SEQ.load(Ordering::SeqCst) != seq {
                    return;
                }
                if crate::ui_state::ui_visible() {
                    return;
                }
                if ui_busy() {
                    // 下载/更新还在跑：等下一轮，别把界面从用户眼前抽走。
                    if attempt >= UI_RECYCLE_MAX_DEFER {
                        log::warn!(
                            "UI RECYCLE: giving up after {attempt} deferrals (download still active)"
                        );
                        return;
                    }
                    schedule_ui_recycle(&app_main, attempt + 1);
                    return;
                }
                let Some(window) = app_main.get_webview_window(MAIN_LABEL) else {
                    return;
                };
                if !window.is_minimized().unwrap_or(false) {
                    return;
                }
                // 快捷键录入会话挂在窗口上；窗口没了必须一起收，否则钩子继续吞键。
                cancel_shortcut_capture(&app_main);
                log::info!("UI RECYCLE: releasing WebView2 while in tray");
                match window.destroy() {
                    Ok(()) => log::info!("UI RECYCLE: main window destroyed"),
                    Err(e) => log::warn!("UI RECYCLE: destroy failed: {e}"),
                }
            }) {
                log::warn!("UI RECYCLE: queue destroy failed: {e}");
            }
        });
    if let Err(e) = spawned {
        log::warn!("UI RECYCLE: spawn failed: {e}");
    }
}

/// 恢复 WebView2 渲染可见性（Windows hide/visible:false 后必须先 SetIsVisible）
pub fn reveal_webview(window: &WebviewWindow) {
    #[cfg(target_os = "windows")]
    {
        let w = window.clone();
        let _ = w.with_webview(move |webview| unsafe {
            let _ = webview.controller().SetIsVisible(true);
        });
    }
}

/// 最小化到托盘：show + minimize + 不占任务栏。
///
/// 不用 `hide()`：隐藏后 WebView2 何时被 Windows 回收不可控。这里改为
/// 最小化后由 [`schedule_ui_recycle`] 主动销毁、点托盘再按需重建。
pub fn minimize_main_to_tray(app: &AppHandle, window: &WebviewWindow, reason: &str) {
    reveal_webview(window);
    let _ = window.show();
    let _ = window.minimize();
    let _ = window.set_skip_taskbar(true);
    // 立即置为隐藏，不等窗口事件；事件广播由 on_window_event 统一发出。
    crate::ui_state::set_window_visible(false);
    log::info!("WINDOW: minimized to tray reason={reason} (skip_taskbar, no hide)");
    schedule_ui_recycle(app, 0);
}

/// 还原主窗口到前台（托盘左键 / 二次启动 / 菜单“打开状态”）。
/// 界面已被回收时先按需重建，重建后等前端就绪自己显示，避免白屏闪一下。
pub fn restore_main_window(app: &AppHandle) {
    // 启动到托盘是一次性启动策略；用户恢复后，后续前端重载不能再次最小化。
    set_boot_to_tray(false);
    cancel_ui_recycle();
    match app.get_webview_window(MAIN_LABEL) {
        Some(window) => {
            let _ = window.set_skip_taskbar(false);
            reveal_webview(&window);
            let _ = window.unminimize();
            let _ = window.show();
            let _ = window.set_focus();
            sync_ui_visibility(app, &window);
        }
        None => match create_main_window(app) {
            Ok(window) => {
                let _ = window.set_skip_taskbar(false);
                let _ = window.set_focus();
                // 不在这里 show：等前端 onMounted 调 reveal_main_on_frontend_ready。
            }
            Err(e) => {
                PENDING_TRAY_REPAIR.store(false, Ordering::SeqCst);
                log::error!("WINDOW: rebuild failed: {e}");
            }
        },
    }
}

/// 前端就绪后显示窗口；若启动策略为托盘则直接 minimize_to_tray。
///
/// 托盘“一键修复”的待办不在这里补发：前端挂载、监听就绪后会主动调
/// [`take_pending_tray_repair`] 取走，避免用 sleep 猜前端什么时候准备好。
pub fn reveal_main_on_frontend_ready(app: &AppHandle) {
    if let Some(window) = app.get_webview_window(MAIN_LABEL) {
        if boot_to_tray() {
            minimize_main_to_tray(app, &window, "frontend_ready");
        } else {
            let _ = window.set_skip_taskbar(false);
            reveal_webview(&window);
            let _ = window.show();
            sync_ui_visibility(app, &window);
        }
    }
}

/// 托盘“一键修复”：界面在时原样广播；界面被回收时先重建，由前端取走待办。
pub fn request_tray_repair(app: &AppHandle) {
    if app.get_webview_window(MAIN_LABEL).is_some() {
        let _ = app.emit("tray-auto-repair", ());
        return;
    }
    log::info!("TRAY: repair requested with UI asleep; rebuilding window first");
    PENDING_TRAY_REPAIR.store(true, Ordering::SeqCst);
    restore_main_window(app);
}

/// 前端挂好 `tray-auto-repair` 监听后调用：取走并清除托盘修复待办。
pub fn take_pending_tray_repair() -> bool {
    PENDING_TRAY_REPAIR.swap(false, Ordering::SeqCst)
}

/// 关闭到托盘：minimize + skip_taskbar（不用 hide）
pub fn attach_main_window_events(app: &AppHandle, window: &WebviewWindow) {
    let app_handle = app.clone();
    let window_ = window.clone();
    window.on_window_event(move |event| match event {
        tauri::WindowEvent::CloseRequested { api, .. } => {
            let minimize = app_handle
                .try_state::<crate::config::manager::ConfigManager>()
                .and_then(|m| m.get_global_settings().ok())
                .map(|s| s.minimize_to_tray)
                .unwrap_or(true);
            if minimize {
                api.prevent_close();
                log::info!("WINDOW: CloseRequested -> minimize_to_tray");
                minimize_main_to_tray(&app_handle, &window_, "close_requested");
            } else {
                // 关窗即退出（托盘仍存活时须主动 exit）
                api.prevent_close();
                crate::ipc::tray::quit_app_public(&app_handle);
            }
        }
        // 最小化/还原/重建都会改变可见性：这是界面该不该刷新界面的唯一判据。
        tauri::WindowEvent::Resized(_) => {
            sync_ui_visibility(&app_handle, &window_);
            // 标题栏“最小化”按钮直接调 window.minimize()，不经过托盘入口。
            // 统一在这里收起任务栏图标并排队回收，否则最常见的收起方式
            // 永远拿不到内存收益。
            if window_.is_minimized().unwrap_or(false) {
                let _ = window_.set_skip_taskbar(true);
                schedule_ui_recycle(&app_handle, 0);
            }
        }
        tauri::WindowEvent::Focused(_) => {
            sync_ui_visibility(&app_handle, &window_);
        }
        _ => {}
    });
}

fn cancel_shortcut_capture(app: &AppHandle) {
    if let Some(session) =
        app.try_state::<crate::bridges::shared::shortcut_capture::ShortcutCaptureSession>()
    {
        let _ = session.cancel();
    }
}

/// 尝试 reload 主窗口；界面已被回收时重建等同刷新。
///
/// 返回值区分“真 reload”和“重建”，否则调用方会把一次重建记成 reload 成功。
pub fn try_reload_main(app: &AppHandle) -> Result<RefreshOutcome, String> {
    cancel_shortcut_capture(app);
    if app.get_webview_window(MAIN_LABEL).is_none() {
        restore_main_window(app);
        if app.get_webview_window(MAIN_LABEL).is_none() {
            return Err("main window rebuild failed".to_string());
        }
        return Ok(RefreshOutcome::Rebuilt);
    }
    restore_main_window(app);
    let window = app
        .get_webview_window(MAIN_LABEL)
        .ok_or_else(|| "main window not found".to_string())?;
    window
        .reload()
        .map_err(|e| format!("WebView2 error: {e}"))?;
    Ok(RefreshOutcome::Reloaded)
}

fn build_main_window(app: &AppHandle) -> Result<WebviewWindow, String> {
    WebviewWindowBuilder::new(app, MAIN_LABEL, WebviewUrl::App("index.html".into()))
        .title("Voice VibeCoding")
        .inner_size(722.0, 520.0)
        .min_inner_size(722.0, 480.0)
        .max_inner_size(960.0, 900.0)
        .resizable(true)
        .maximizable(false)
        .center()
        .decorations(false)
        .visible(false)
        .build()
        .map_err(|e| e.to_string())
}

/// 新建主窗口（托盘恢复、白屏自救共用）。
///
/// 建好后保持隐藏，等前端 onMounted 调 `reveal_main_on_frontend_ready` 再显示，
/// 避免空白页闪一下；前端若没起来，[`UI_FIRST_SHOW_FALLBACK`] 后兜底显示。
fn create_main_window(app: &AppHandle) -> Result<WebviewWindow, String> {
    let window = build_main_window(app)?;
    attach_main_window_events(app, &window);
    reveal_webview(&window);
    sync_ui_visibility(app, &window);
    webview_guard::on_recreated();

    let app_bg = app.clone();
    std::thread::Builder::new()
        .name("ui-first-show".into())
        .spawn(move || {
            std::thread::sleep(UI_FIRST_SHOW_FALLBACK);
            if boot_to_tray() {
                return;
            }
            let Some(window) = app_bg.get_webview_window(MAIN_LABEL) else {
                return;
            };
            if window.is_visible().unwrap_or(false) {
                return;
            }
            log::warn!("WINDOW: frontend did not reveal in time; showing anyway");
            let _ = window.set_skip_taskbar(false);
            reveal_webview(&window);
            let _ = window.show();
        })
        .ok();

    Ok(window)
}

/// 销毁并重建主窗口（reload 无法复活僵尸 WebView2 时的唯一手段）
pub fn recreate_main_window(app: &AppHandle) -> Result<(), String> {
    cancel_shortcut_capture(app);
    if let Some(old) = app.get_webview_window(MAIN_LABEL) {
        log::warn!("WEBVIEW RECOVERY: destroying zombie main window");
        old.destroy()
            .map_err(|e| format!("destroy main window failed: {e}"))?;
        // 给 WebView2 进程一点时间退出
        std::thread::sleep(Duration::from_millis(300));
    }

    create_main_window(app)?;
    log::info!("WEBVIEW RECOVERY: main window recreated");
    Ok(())
}

/// reload → recreate 两级恢复
pub fn apply_health_action(app: &AppHandle, action: HealthAction) {
    match action {
        HealthAction::None => {}
        HealthAction::Reload => {
            log::warn!("WEBVIEW GUARD: reloading main window (rendering suspected dead)");
            match try_reload_main(app) {
                Ok(RefreshOutcome::Reloaded) => log::info!("WEBVIEW GUARD: reload succeeded"),
                Ok(RefreshOutcome::Rebuilt) => {
                    log::info!("WEBVIEW GUARD: window was absent; rebuilt instead of reloading");
                }
                Err(e) => {
                    log::error!("WEBVIEW GUARD: failed to reload: {e}");
                    webview_guard::note_reload_failed();
                    if webview_guard::needs_recreate() {
                        if let Err(e2) = recreate_main_window(app) {
                            log::error!("WEBVIEW GUARD: recreate after reload failed: {e2}");
                        }
                    }
                }
            }
        }
        HealthAction::Recreate => {
            log::warn!("WEBVIEW GUARD: recreating main window (reload ineffective)");
            if let Err(e) = recreate_main_window(app) {
                log::error!("WEBVIEW GUARD: recreate failed: {e}");
            }
        }
    }
}

/// 托盘“刷新界面”：先 reload，失败则 recreate。
/// 界面已被回收时直接重建，等于刷新。
pub fn manual_refresh_ui(app: &AppHandle) {
    log::info!("TRAY: manual refresh UI requested");
    match try_reload_main(app) {
        Ok(RefreshOutcome::Reloaded) => log::info!("TRAY: refresh reload succeeded"),
        Ok(RefreshOutcome::Rebuilt) => {
            log::info!("TRAY: refresh rebuilt the recycled window");
        }
        Err(e) => {
            log::warn!("TRAY: refresh reload failed ({e}), trying recreate");
            webview_guard::note_reload_failed();
            if let Err(e2) = recreate_main_window(app) {
                log::error!("TRAY: refresh recreate failed: {e2}");
            }
        }
    }
}

/// 托盘「重启软件」：清理桥接/HID/音频后 relaunch。
/// 钩子 stop 单一入口之一（另两处：进程退出 cleanup、托盘退出 quit_app）。
pub fn restart_application(app: &AppHandle) {
    log::info!("TRAY: restarting application");
    cancel_shortcut_capture(app);
    if let Some(runtime) =
        app.try_state::<std::sync::Arc<crate::bridges::xiaomi::connect::XiaomiRuntime>>()
    {
        runtime.request_stop();
    }
    crate::bridges::xiaomi::winuhid_env::cancel_runtime_reprobe();
    crate::bridges::xiaomi::key_mapping::release_voice_resources();
    crate::bridges::xiaomi::hid_report_tap::stop_and_join();
    crate::bridges::xiaomi::special_keys::stop_special_key_hook();
    crate::audio::pcm_router::stop_audio_router_process();
    std::thread::sleep(std::time::Duration::from_millis(150));
    app.restart();
}

#[cfg(test)]
mod tests {
    #[test]
    fn main_label_constant() {
        assert_eq!(super::MAIN_LABEL, "main");
    }
}
