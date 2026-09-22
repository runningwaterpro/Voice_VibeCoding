//! 录入就绪契约：探针不进 publish、bump 必须 overlap+settle、UI 不得先亮。
//!
//! 运行: cargo test --test capture_ready_contract

#[test]
fn probe_vk_never_feeds_engine() {
    let src = include_str!("../src/bridges/shared/shortcut_capture.rs");
    let feed = src
        .split("pub fn feed_capture_key")
        .nth(1)
        .and_then(|s| s.split("\npub fn ").next())
        .expect("feed_capture_key body");
    assert!(
        feed.contains("PROBE_VK"),
        "feed_capture_key must early-return PROBE_VK so F24 is never published"
    );
}

#[test]
fn probe_failure_may_log_but_not_restart_or_hard_fail() {
    // 第一性：SendInput F24 探针恒假阴；禁止用它驱动 restart（会触发线程竞态）。
    // 录入主路径 = WebView：start 不得因 armed/探针失败 return Err。
    let src = include_str!("../src/bridges/shared/shortcut_capture.rs");
    let start = src
        .split("pub fn start(&self, app: AppHandle)")
        .nth(1)
        .and_then(|s| s.split("\n    pub fn take_result").next())
        .expect("start body");
    assert!(
        !start.contains("restart_special_key_hook"),
        "start must not auto-restart hook on probe false-negative"
    );
    assert!(
        !start.contains("return Err"),
        "start must not hard-fail capture when hook not armed (WebView is primary)"
    );
}

#[test]
fn hook_loop_exit_must_not_steal_new_thread_handle() {
    // 根因：旧线程退出时 store_hook(null)+Unhook(load_hook()) 会卸掉新线程的钩子。
    let src = include_str!("../src/bridges/xiaomi/special_keys.rs");
    let exit = src
        .split("hook loop exit")
        .nth(0)
        .and_then(|s| s.rsplit("while RUNNING").next())
        .or_else(|| {
            src.rfind("let mine = load_hook()")
                .map(|i| &src[i..])
        })
        .expect("exit path");
    assert!(
        exit.contains("compare_exchange") || exit.contains("mine"),
        "exit must only unhook its own handle, not blindly store_hook(null)+Unhook(load_hook)"
    );
    assert!(
        !exit.contains("store_hook(HHOOK(std::ptr::null_mut()));\n        if !hook.is_invalid()"),
        "old exit pattern that nulls global then unhooks load_hook() is forbidden"
    );
}

#[test]
fn hook_loop_exit_uses_mine_handle() {
    let src = include_str!("../src/bridges/xiaomi/special_keys.rs");
    assert!(
        src.contains("let mine = load_hook()"),
        "exit must capture its own hook handle"
    );
    assert!(
        src.contains("compare_exchange"),
        "exit must CAS-clear HOOK_PTR only if still owner"
    );
}

#[test]
fn start_must_not_teardown_running_hook() {
    let src = include_str!("../src/bridges/xiaomi/special_keys.rs");
    let start = src
        .split("pub fn start_special_key_hook()")
        .nth(1)
        .and_then(|s| s.split("pub fn stop_special_key_hook").next())
        .or_else(|| {
            src.split("pub fn start_special_key_hook()")
                .nth(1)
                .and_then(|s| s.split("fn stop_and_join").next())
        })
        .expect("start body");
    assert!(
        start.contains("if RUNNING.load") && start.contains("return"),
        "start must return early when hook thread already running"
    );
}

#[test]
fn bump_must_set_overlap_before_unhook() {
    let src = include_str!("../src/bridges/xiaomi/special_keys.rs");
    let body = src
        .split("msg.message == WM_BUMP_HOOK_FRONT")
        .nth(1)
        .and_then(|s| s.split("TranslateMessage").next())
        .expect("bump handler");
    let set_pos = body.find("SetWindowsHookExW").expect("Set");
    let unhook_pos = body.find("UnhookWindowsHookEx").expect("Unhook");
    assert!(
        set_pos < unhook_pos,
        "overlap: must Set new hook before Unhook old"
    );
    assert!(body.contains("mark_handled"), "bump must mark_handled");
}

#[test]
fn bump_settle_uses_hook_bump_not_blind_sleep() {
    let src = include_str!("../src/bridges/xiaomi/special_keys.rs");
    let settle = src
        .split("pub fn bump_hook_to_front_and_settle")
        .nth(1)
        .and_then(|s| s.split("pub fn is_hook_running").next())
        .expect("settle fn");
    assert!(
        settle.contains("hook_bump::wait_for") || settle.contains("wait_for("),
        "settle must wait on generation, not sleep"
    );
}

#[test]
fn ensure_hook_must_not_force_bump_every_capture() {
    let src = include_str!("../src/bridges/xiaomi/special_keys.rs");
    let ensure = src
        .split("pub fn ensure_hook_for_capture")
        .nth(1)
        .and_then(|s| s.split("pub fn restart_special_key_hook").next())
        .expect("ensure body");
    assert!(
        !ensure.contains("bump_hook_to_front"),
        "ensure must not blind-bump every capture"
    );
}

#[test]
fn leak_chord_must_not_fail_before_restart() {
    let src = include_str!("../src/bridges/shared/shortcut_capture.rs");
    assert!(
        src.contains("LEAK_RESTART_IN_FLIGHT") || src.contains("restart in flight"),
        "must absorb chord keys while hook restart in flight"
    );
    assert!(
        src.contains("restart_special_key_hook"),
        "leak recovery must full-restart hook thread"
    );
}

#[test]
fn stop_only_at_app_lifecycle_not_reconnect() {
    // 钩子 stop 单一所有权：断线重连循环不得 stop（会与新连接 start 竞态）。
    let src = include_str!("../src/ipc/commands.rs");
    let start = src
        .split("fn xiaomi_reconnect_loop")
        .nth(1)
        .or_else(|| src.split("monitor_connection").nth(1))
        .expect("reconnect region");
    let region = start.split("fn wait_interruptible").next().unwrap_or(start);
    assert!(
        !region.contains("stop_special_key_hook"),
        "reconnect loop must not stop hook; only exit/tray/restart may"
    );
}

#[test]
fn frontend_records_chord_from_webview_keydown() {
    // 第一性：钩子可能收不到键，但 WebView 能收到 keydown —— 必须能从前端录。
    for rel in [
        "src/components/KeyMappingStage.vue",
        "src/components/KeyBindingEditor.vue",
    ] {
        let src = std::fs::read_to_string(format!("../{rel}")).expect(rel);
        assert!(
            src.contains("chordFromEvent"),
            "{rel}: must build chord from KeyboardEvent"
        );
        let block = src
            .split("function blockBrowserKeysDuringCapture")
            .nth(1)
            .and_then(|s| s.split("\nfunction ").next())
            .or_else(|| {
                src.split("function blockBrowserKeysDuringCapture")
                    .nth(1)
                    .and_then(|s| s.split("\nasync function ").next())
            })
            .expect("block fn");
        assert!(
            block.contains("onCaptured"),
            "{rel}: keydown path must call onCaptured"
        );
    }
}
