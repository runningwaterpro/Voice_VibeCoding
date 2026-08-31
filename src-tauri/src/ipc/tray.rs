//! 系统托盘 — 左键还原窗口，右键菜单

use tauri::{
    image::Image,
    menu::{Menu, MenuBuilder, MenuItemBuilder, PredefinedMenuItem, SubmenuBuilder},
    tray::{MouseButton, TrayIcon, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager,
};
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::OnceLock;

fn quit_app(app: &AppHandle) {
    // 先停托盘状态线程，再停桥接 / HID Tap / 钩子 / 音频子进程
    stop_tray_state();
    if let Some(runtime) =
        app.try_state::<std::sync::Arc<crate::bridges::xiaomi::connect::XiaomiRuntime>>()
    {
        runtime.request_stop();
    }
    crate::bridges::xiaomi::hid_report_tap::stop_and_join();
    crate::bridges::xiaomi::special_keys::stop_special_key_hook();
    crate::audio::pcm_router::stop_audio_router_process();
    // 给后台线程一点时间退出 accept/GetMessage
    std::thread::sleep(std::time::Duration::from_millis(150));
    app.exit(0);
}

/// 供 IPC `quit_application` 调用
pub fn quit_app_public(app: &AppHandle) {
    quit_app(app);
}

// ============================================================
// 托盘三态语音就绪指示
//
//   Initializing → 呼吸灯（正弦透明度淡入淡出）
//   Success      → 正常图标
//   Failed       → 正常图标 + 右下角红色叹号徽标
//
// 状态机由 input_session 的会话生命周期驱动；呼吸 worker 为
// 常驻守护线程，只在 Initializing 阶段渲染帧，其余时间空转。
// 不在 LL 键盘钩子热路径上，零时序污染。
// ============================================================

#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum TrayPhase {
    Initializing = 0,
    Success = 1,
    Failed = 2,
}

impl TrayPhase {
    fn from_u8(v: u8) -> Self {
        match v {
            1 => TrayPhase::Success,
            2 => TrayPhase::Failed,
            _ => TrayPhase::Initializing,
        }
    }
}

static PHASE: AtomicU8 = AtomicU8::new(TrayPhase::Initializing as u8);
static BREATH_RUNNING: AtomicBool = AtomicBool::new(false);

/// 正常图标（Success）—— 512px ARGB
fn ready_icon() -> Image<'static> {
    static READY: OnceLock<Image<'static>> = OnceLock::new();
    READY.get_or_init(|| {
        Image::from_bytes(include_bytes!("../../icons/tray-icon.png"))
            .unwrap_or_else(|_| Image::from_bytes(include_bytes!("../../icons/32x32.png")).unwrap())
    })
    .clone()
}

/// 失败图标（正常 + 右下角红圆白叹号）—— 512px ARGB
fn error_icon() -> Image<'static> {
    static ERR: OnceLock<Image<'static>> = OnceLock::new();
    ERR.get_or_init(|| {
        Image::from_bytes(include_bytes!("../../icons/tray-icon-error.png"))
            .unwrap_or_else(|_| ready_icon())
    })
    .clone()
}

/// 呼吸动画的基像素：把正常图标降采样到 128² 的 RGBA 缓冲（每帧复用副本）。
fn breath_base() -> &'static image::RgbaImage {
    static BASE: OnceLock<image::RgbaImage> = OnceLock::new();
    BASE.get_or_init(|| {
        let img = image::load_from_memory(include_bytes!("../../icons/tray-icon.png"))
            .expect("decode tray-icon for breathing");
        image::imageops::resize(
            &img.into_rgba8(),
            128,
            128,
            image::imageops::FilterType::Lanczos3,
        )
    })
}

fn tooltip_for(phase: TrayPhase) -> &'static str {
    match phase {
        TrayPhase::Initializing => "Voice VibeCoding（正在初始化…）",
        TrayPhase::Success => "Voice VibeCoding（已就绪）",
        TrayPhase::Failed => "Voice VibeCoding（初始化失败）",
    }
}

/// 把图标 + tooltip 应用到托盘（主线程执行）
fn apply_icon(app: &AppHandle, icon: Image<'static>, phase: TrayPhase) {
    let app = app.clone();
    let _ = app.run_on_main_thread(move || {
        if let Some(tray) = app.tray_by_id("main") {
            let _ = tray.set_icon(Some(icon));
            let _ = tray.set_tooltip(Some(tooltip_for(phase)));
        }
    });
}

/// 呼吸守护线程。只在 Initializing 渲染帧；其它阶段空转。退出靠 BREATH_RUNNING=false。
fn breath_worker(app: AppHandle) {
    let base = breath_base().clone(); // 128x128 RgbaImage
    let (w, h) = base.dimensions();
    let mut t = 0.0_f32;
    let step = 0.030_f32; // ~33fps
    while BREATH_RUNNING.load(Ordering::Acquire) {
        let phase = TrayPhase::from_u8(PHASE.load(Ordering::Acquire));
        if phase == TrayPhase::Initializing {
            // 呼吸：周期 ~1.6s，alpha 35%→100%
            let a = 0.35 + 0.65 * ((t * std::f32::consts::TAU / 1.6).sin().abs());
            let ai = (255.0 * a) as u8;
            let mut frame = base.clone();
            for p in frame.pixels_mut() {
                p.0[3] = ((p.0[3] as f32) * (ai as f32) / 255.0) as u8;
            }
            let img = Image::new_owned(frame.into_raw(), w, h);
            let app2 = app.clone();
            let _ = app2.run_on_main_thread(move || {
                if let Some(tray) = app2.tray_by_id("main") {
                    let _ = tray.set_icon(Some(img));
                }
            });
            t = (t + step) % 1.6;
        } else {
            t = 0.0;
        }
        std::thread::sleep(std::time::Duration::from_millis(30));
    }
}

/// 启动呼吸守护线程（幂等；首次调用时拉起）
fn ensure_breath_worker(app: &AppHandle) {
    if BREATH_RUNNING.swap(true, Ordering::AcqRel) {
        return;
    }
    let app = app.clone();
    let _ = std::thread::Builder::new()
        .name("tray-breath".into())
        .spawn(move || breath_worker(app));
}

/// 退出前停止呼吸线程
pub fn stop_tray_state() {
    BREATH_RUNNING.store(false, Ordering::Release);
}

/// 设置托盘状态（对外主入口）
pub fn set_tray_phase(app: &AppHandle, phase: TrayPhase) {
    let prev = TrayPhase::from_u8(PHASE.swap(phase as u8, Ordering::AcqRel));
    if prev == phase {
        return;
    }
    match phase {
        TrayPhase::Initializing => {
            // 呼吸 worker 自会渲染；先确保它在跑，并把 tooltip 切到"正在初始化"
            ensure_breath_worker(app);
            let app2 = app.clone();
            let _ = app2.run_on_main_thread(move || {
                if let Some(tray) = app2.tray_by_id("main") {
                    let _ = tray.set_tooltip(Some(tooltip_for(TrayPhase::Initializing)));
                }
            });
        }
        TrayPhase::Success => {
            ensure_breath_worker(app); // 确保线程在跑（后续可复用）
            apply_icon(app, ready_icon(), TrayPhase::Success);
        }
        TrayPhase::Failed => {
            ensure_breath_worker(app);
            apply_icon(app, error_icon(), TrayPhase::Failed);
        }
    }
}

fn build_tray_menu(app: &AppHandle) -> tauri::Result<Menu<tauri::Wry>> {
    // 对齐 Python xiaomi_main 托盘：打开状态 / 按键与语音设置 / 重启桥接 / 退出
    let restore = MenuItemBuilder::with_id("restore", "打开状态").build(app)?;
    let settings = MenuItemBuilder::with_id("xiaomi_settings", "按键与语音设置").build(app)?;
    let restart = MenuItemBuilder::with_id("restart_bridge", "重启桥接").build(app)?;
    let refresh = MenuItemBuilder::with_id("refresh_ui", "刷新界面（白屏自救）").build(app)?;
    let restart_app = MenuItemBuilder::with_id("restart_app", "重启软件").build(app)?;
    let separator1 = PredefinedMenuItem::separator(app)?;

    let xiaomi_connect = MenuItemBuilder::with_id("xiaomi_connect", "连接小米遥控器").build(app)?;
    let xiaomi_disconnect =
        MenuItemBuilder::with_id("xiaomi_disconnect", "断开小米遥控器").build(app)?;
    let xiaomi_submenu = SubmenuBuilder::new(app, "小米遥控器")
        .item(&xiaomi_connect)
        .item(&xiaomi_disconnect)
        .build()?;

    let t1_connect = MenuItemBuilder::with_id("t1_connect", "连接 T1 遥控器").build(app)?;
    let t1_disconnect = MenuItemBuilder::with_id("t1_disconnect", "断开 T1 遥控器").build(app)?;
    let t1_submenu = SubmenuBuilder::new(app, "T1 遥控器")
        .item(&t1_connect)
        .item(&t1_disconnect)
        .build()?;

    let v60_connect = MenuItemBuilder::with_id("hanvon_connect", "连接 V60 语音笔").build(app)?;
    let v60_disconnect =
        MenuItemBuilder::with_id("hanvon_disconnect", "断开 V60 语音笔").build(app)?;
    let v60_submenu = SubmenuBuilder::new(app, "汉王 V60")
        .item(&v60_connect)
        .item(&v60_disconnect)
        .build()?;

    let separator2 = PredefinedMenuItem::separator(app)?;
    let quit = MenuItemBuilder::with_id("quit", "退出").build(app)?;

    MenuBuilder::new(app)
        .item(&restore)
        .item(&settings)
        .item(&restart)
        .item(&refresh)
        .item(&restart_app)
        .item(&separator1)
        .item(&xiaomi_submenu)
        .item(&t1_submenu)
        .item(&v60_submenu)
        .item(&separator2)
        .item(&quit)
        .build()
}

fn on_menu_event(app: &AppHandle, id: &str) {
    match id {
        "restore" | "show" => crate::webview_recovery::restore_main_window(app),
        "refresh_ui" => crate::webview_recovery::manual_refresh_ui(app),
        "restart_app" => crate::webview_recovery::restart_application(app),
        "xiaomi_settings" => {
            crate::webview_recovery::restore_main_window(app);
            let _ = app.emit("navigate", "/xiaomi");
        }
        "restart_bridge" => {
            let app = app.clone();
            std::thread::spawn(move || {
                let Some(state) = app.try_state::<crate::bridges::BridgeState>() else {
                    return;
                };
                let Some(config_manager) =
                    app.try_state::<crate::config::manager::ConfigManager>()
                else {
                    return;
                };
                if let Err(e) =
                    crate::ipc::commands::restart_xiaomi_bridge_inner(&app, &state, &config_manager)
                {
                    log::warn!("Tray restart bridge failed: {e}");
                }
            });
        }
        "quit" => quit_app(app),
        "xiaomi_connect" => {
            log::info!("Tray: connecting Xiaomi");
            let app = app.clone();
            std::thread::spawn(move || {
                let Some(state) = app.try_state::<crate::bridges::BridgeState>() else {
                    return;
                };
                let Some(config_manager) =
                    app.try_state::<crate::config::manager::ConfigManager>()
                else {
                    return;
                };
                if let Err(e) =
                    crate::ipc::commands::restart_xiaomi_bridge_inner(&app, &state, &config_manager)
                {
                    log::warn!("Tray connect (restart) failed: {e}");
                }
            });
        }
        "xiaomi_disconnect" => {
            log::info!("Tray: disconnecting Xiaomi");
            if let Some(runtime) =
                app.try_state::<std::sync::Arc<crate::bridges::xiaomi::connect::XiaomiRuntime>>()
            {
                runtime.request_stop();
            }
            if let Some(state) = app.try_state::<crate::bridges::BridgeState>() {
                state.update_status(
                    crate::bridges::BridgeType::Xiaomi,
                    crate::bridges::BridgeStatus::Disconnected,
                );
            }
        }
        "t1_connect" => log::info!("Tray: connecting T1"),
        "t1_disconnect" => log::info!("Tray: disconnecting T1"),
        "hanvon_connect" => log::info!("Tray: connecting V60"),
        "hanvon_disconnect" => log::info!("Tray: disconnecting V60"),
        _ => {}
    }
}

/// 配置系统托盘图标和菜单；返回的 TrayIcon 必须由调用方 keep-alive（manage）
pub fn setup_tray(app: &AppHandle) -> Result<TrayIcon, Box<dyn std::error::Error>> {
    let menu = build_tray_menu(app)?;

    // 优先用专用托盘图；失败再退回窗口默认图标
    let icon = Image::from_bytes(include_bytes!("../../icons/tray-icon.png"))
        .or_else(|_| Image::from_bytes(include_bytes!("../../icons/32x32.png")))
        .or_else(|_| {
            app.default_window_icon()
                .cloned()
                .ok_or_else(|| tauri::Error::FailedToReceiveMessage)
        })?;

    let tray = TrayIconBuilder::with_id("main")
        .icon(icon)
        .icon_as_template(false)
        .menu(&menu)
        .tooltip("Voice VibeCoding（正在初始化…）")
        // 左键还原窗口；右键弹出菜单（Windows 默认）
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| {
            on_menu_event(app, event.id().as_ref());
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: tauri::tray::MouseButtonState::Up,
                ..
            } = event
            {
                crate::webview_recovery::restore_main_window(tray.app_handle());
            }
        })
        .build(app)?;

    // 托盘就绪即进入"正在初始化"呼吸态，等 input_session 就绪回调切到 Success/Failed
    PHASE.store(TrayPhase::Initializing as u8, Ordering::Release);
    ensure_breath_worker(app.handle());

    log::info!("System tray icon created");
    Ok(tray)
}
