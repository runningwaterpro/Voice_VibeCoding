//! Voice timing and first-packet contracts.

use remote_bridge_hub_lib::bridges::xiaomi::voice_pcm::{
    ping_deadline_secs, ping_retry_interval_ms,
};

#[test]
fn ping_retry_interval_is_aggressive_for_cold_start() {
    assert!(
        ping_retry_interval_ms() <= 20,
        "PING retry should be <=20ms, got {}",
        ping_retry_interval_ms()
    );
}

#[test]
fn ping_deadline_keeps_bounded_wait() {
    assert_eq!(ping_deadline_secs(), 4);
}
