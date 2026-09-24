use remote_bridge_hub_lib::bridges::xiaomi::voice_session::{
    VoiceEvent, VoiceSession, VoiceSessionAdapters,
};
use std::cell::RefCell;
use std::rc::Rc;

struct FakeVoiceAdapters {
    calls: Rc<RefCell<Vec<String>>>,
}

impl FakeVoiceAdapters {
    fn new() -> (Self, Rc<RefCell<Vec<String>>>) {
        let calls = Rc::new(RefCell::new(Vec::new()));
        (
            Self {
                calls: Rc::clone(&calls),
            },
            calls,
        )
    }
}

impl VoiceSessionAdapters for FakeVoiceAdapters {
    fn prepare_audio(&mut self) -> Result<(), String> {
        self.calls.borrow_mut().push("prepare".into());
        Ok(())
    }

    fn clear_audio(&mut self) -> Result<(), String> {
        self.calls.borrow_mut().push("clear".into());
        Ok(())
    }

    fn end_audio(&mut self) -> Result<(), String> {
        self.calls.borrow_mut().push("end".into());
        Ok(())
    }

    fn press_hotkey(&mut self, chord: &[u16]) -> Result<(), String> {
        self.calls.borrow_mut().push(format!("down:{chord:?}"));
        Ok(())
    }

    fn release_hotkey(&mut self, chord: &[u16]) -> Result<(), String> {
        self.calls.borrow_mut().push(format!("up:{chord:?}"));
        Ok(())
    }

    fn push_audio(&mut self, _frame: &[u8]) -> Result<(), String> {
        self.calls.borrow_mut().push("audio".into());
        Ok(())
    }

    fn release_all(&mut self) -> Result<(), String> {
        self.calls.borrow_mut().push("release_all".into());
        Ok(())
    }
}

fn ready_session() -> (VoiceSession<FakeVoiceAdapters>, Rc<RefCell<Vec<String>>>) {
    let (adapters, calls) = FakeVoiceAdapters::new();
    let mut session = VoiceSession::new(42, adapters);
    session.handle(VoiceEvent::ObserveReadiness {
        worker_alive: true,
        device_connected: true,
        voice_channel: true,
        audio_ready: true,
        winuhid_ready: true,
    });
    (session, calls)
}

#[test]
fn production_seam_orders_press_and_tracks_first_packet() {
    let (mut session, calls) = ready_session();
    let pressed = session.handle(VoiceEvent::press(vec![0xA2, 0x44]));
    assert!(pressed.voice_ready);
    assert!(!pressed.first_audio_packet);
    assert_eq!(
        calls.borrow().clone(),
        vec![
            "prepare".to_string(),
            "down:[162, 68]".to_string(),
            "clear".to_string(),
        ]
    );

    let first_packet = session.handle(VoiceEvent::AudioFrame(vec![1, 0, 2, 0]));
    assert!(first_packet.first_audio_packet);
    assert_eq!(
        first_packet.audio,
        remote_bridge_hub_lib::bridges::xiaomi::voice_session::AudioOutcome::FirstPacket
    );
}

#[test]
fn production_seam_rejects_stale_events() {
    let (mut session, _calls) = ready_session();
    let before = session.snapshot();
    let snapshot = session.handle_from(41, VoiceEvent::press(vec![0xA5]));
    assert_eq!(snapshot, before);
    assert!(!snapshot.pressed);
}

#[test]
fn production_seam_releases_on_disconnect() {
    let (mut session, calls) = ready_session();
    session.handle(VoiceEvent::press(vec![0xA5]));
    let snapshot = session.handle(VoiceEvent::Disconnect);
    assert!(!snapshot.pressed);
    assert!(snapshot.resources_clean);
    assert!(calls.borrow().iter().any(|call| call == "release_all"));
}
