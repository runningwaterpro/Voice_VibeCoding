//! 对齐 Python Raw Input fallback：HID Tap 未启动时，用设备过滤的 Raw Input 映射

use crate::bridges::xiaomi::connect::XiaomiRuntime;
use crate::bridges::xiaomi::key_log::KeyEmitGate;
use std::sync::Arc;
use tauri::AppHandle;

/// Raw Input fallback is intentionally disabled until a verified RC003 device
/// identity can be obtained. A keyboard event without that identity is not an
/// RC003 event and must never drive a remote mapping.
pub fn raw_mapping_fallback_allowed() -> bool {
    false
}

/// Do not start the unverified Raw Input fallback in production.
///
/// HID Tap is the production input path. Keeping this function as a no-op
/// preserves the call site while making the safety boundary explicit.
pub fn maybe_start_raw_mapping(
    _app: AppHandle,
    _runtime: Arc<XiaomiRuntime>,
    _gate: Arc<KeyEmitGate>,
    _hid_tap_started: bool,
) {
    log::warn!("XIAOMI RAW MAPPING disabled: no verified RC003 device identity");
}

#[cfg(test)]
mod tests {
    use super::raw_mapping_fallback_allowed;

    #[test]
    fn raw_input_fallback_is_fail_closed_without_device_identity() {
        assert!(!raw_mapping_fallback_allowed());
    }
}
