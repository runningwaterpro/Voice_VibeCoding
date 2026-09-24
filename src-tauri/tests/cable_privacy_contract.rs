#[test]
fn cable_setup_script_is_read_only_for_system_audio() {
    let script = include_str!("../assets/xiaomi/configure-xiaomi-audio.ps1");
    for forbidden in [
        "SetDefaultCapture",
        "SetDefaultEndpoint",
        "CapabilityAccessManager",
        "ConsentStore",
        "Set-ItemProperty",
    ] {
        assert!(
            !script.contains(forbidden),
            "CABLE setup must not mutate system audio policy: {forbidden}"
        );
    }
}

#[test]
fn installer_success_requires_observed_endpoints() {
    use remote_bridge_hub_lib::audio::vb_cable::installer_observed_ready;
    assert!(!installer_observed_ready(Some(0), false, false));
    assert!(!installer_observed_ready(Some(3010), true, false));
    assert!(installer_observed_ready(Some(1), true, true));
}
