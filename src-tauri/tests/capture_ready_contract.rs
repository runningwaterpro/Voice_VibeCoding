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
fn probe_failure_must_not_hard_block_capture() {
    let src = include_str!("../src/bridges/shared/shortcut_capture.rs");
    assert!(
        src.contains("starting capture") || src.contains("leak health armed"),
        "probe fail must continue into capture, not hard Err"
    );
    assert!(
        !src.contains("return Err(\n                    \"keyboard") && !src.contains("无法捕获键盘"),
        "must not hard-fail start solely on probe"
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
fn frontend_must_not_set_capturing_before_start_returns() {
    for rel in [
        "src/components/KeyMappingStage.vue",
        "src/components/KeyBindingEditor.vue",
    ] {
        let src = std::fs::read_to_string(format!("../{rel}")).expect(rel);
        let start_idx = src
            .find("async function startCapture")
            .or_else(|| src.find("async function startEdit"))
            .expect("start fn");
        let chunk = &src[start_idx..src.len().min(start_idx + 900)];
        let capturing_set = chunk.find("capturing.value = true").expect("set capturing");
        let invoke_pos = chunk
            .find("capture_shortcut_start")
            .expect("invoke start");
        assert!(
            invoke_pos < capturing_set,
            "{rel}: capturing must be set AFTER successful capture_shortcut_start"
        );
    }
}
