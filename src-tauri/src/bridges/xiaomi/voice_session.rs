//! The production-facing RC003 voice session seam.
//!
//! GATT decoding and platform adapters feed events into `VoiceSession`; callers
//! observe only `VoiceSnapshot`. This keeps press/release ordering and the
//! first-packet fact in one place instead of rebuilding them in the UI.

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum VoiceEvent {
    Press {
        chord: Vec<u16>,
    },
    Release,
    AudioFrame(Vec<u8>),
    Disconnect,
    Shutdown,
    ObserveReadiness {
        worker_alive: bool,
        device_connected: bool,
        voice_channel: bool,
        audio_ready: bool,
        winuhid_ready: bool,
    },
}

impl VoiceEvent {
    pub fn press(chord: Vec<u16>) -> Self {
        Self::Press { chord }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum InjectionOutcome {
    NotAttempted,
    Ready,
    Failed,
    Released,
    Stale,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AudioOutcome {
    Unknown,
    Ready,
    FirstPacket,
    Failed,
    Stopped,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum VoicePhase {
    Idle,
    Connecting,
    Ready,
    Pressed,
    Recovering,
    Disconnected,
    Stopped,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PrimaryAction {
    None,
    ConnectRemote,
    RepairWinUHid,
    RepairAudio,
    Retry,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct VoiceSnapshot {
    pub generation: u64,
    pub worker_alive: bool,
    pub device_connected: bool,
    pub voice_channel: bool,
    pub voice_ready: bool,
    pub first_audio_packet: bool,
    pub pressed: bool,
    pub injection: InjectionOutcome,
    pub audio: AudioOutcome,
    pub phase: VoicePhase,
    pub status_code: String,
    pub detail: String,
    pub primary_action: PrimaryAction,
    pub resources_clean: bool,
}

static LATEST_SNAPSHOT: Mutex<Option<VoiceSnapshot>> = Mutex::new(None);

pub fn publish_snapshot(snapshot: &VoiceSnapshot) {
    *LATEST_SNAPSHOT.lock() = Some(snapshot.clone());
}

pub fn latest_snapshot() -> Option<VoiceSnapshot> {
    LATEST_SNAPSHOT.lock().clone()
}

pub fn clear_latest_snapshot() {
    *LATEST_SNAPSHOT.lock() = None;
}

/// Platform operations used by the session. Implementations may be backed by
/// WinUHid, PCM, or test fakes; the session owns their ordering.
pub trait VoiceSessionAdapters {
    fn prepare_audio(&mut self) -> Result<(), String>;
    fn clear_audio(&mut self) -> Result<(), String>;
    fn end_audio(&mut self) -> Result<(), String>;
    fn press_hotkey(&mut self, chord: &[u16]) -> Result<(), String>;
    fn release_hotkey(&mut self, chord: &[u16]) -> Result<(), String>;
    fn push_audio(&mut self, frame: &[u8]) -> Result<(), String>;
    fn release_all(&mut self) -> Result<(), String>;
}

pub struct VoiceSession<A> {
    generation: u64,
    chord: Vec<u16>,
    worker_alive: bool,
    device_connected: bool,
    voice_channel: bool,
    audio_ready: bool,
    winuhid_ready: bool,
    pressed: bool,
    first_audio_packet: bool,
    injection: InjectionOutcome,
    audio: AudioOutcome,
    phase: VoicePhase,
    status_code: String,
    detail: String,
    primary_action: PrimaryAction,
    resources_clean: bool,
    adapters: A,
}

pub fn snapshot_from_readiness(
    generation: u64,
    worker_alive: bool,
    device_connected: bool,
    voice_channel: bool,
    audio_ready: bool,
    winuhid_ready: bool,
) -> VoiceSnapshot {
    let voice_ready =
        worker_alive && device_connected && voice_channel && audio_ready && winuhid_ready;
    let (phase, status_code, detail, primary_action) = if voice_ready {
        (
            VoicePhase::Ready,
            "ready",
            "语音环境已就绪",
            PrimaryAction::None,
        )
    } else if !worker_alive || !device_connected {
        (
            VoicePhase::Connecting,
            "not_ready",
            "RC003 尚未连接",
            PrimaryAction::ConnectRemote,
        )
    } else if !voice_channel {
        (
            VoicePhase::Recovering,
            "voice_channel_missing",
            "ATVV 语音通道尚未就绪",
            PrimaryAction::Retry,
        )
    } else if !winuhid_ready {
        (
            VoicePhase::Recovering,
            "winuhid_missing",
            "WinUHid 尚未就绪",
            PrimaryAction::RepairWinUHid,
        )
    } else {
        (
            VoicePhase::Recovering,
            "audio_missing",
            "音频路由尚未就绪",
            PrimaryAction::RepairAudio,
        )
    };
    VoiceSnapshot {
        generation,
        worker_alive,
        device_connected,
        voice_channel,
        voice_ready,
        first_audio_packet: false,
        pressed: false,
        injection: InjectionOutcome::NotAttempted,
        audio: if audio_ready {
            AudioOutcome::Ready
        } else {
            AudioOutcome::Unknown
        },
        phase,
        status_code: status_code.into(),
        detail: detail.into(),
        primary_action,
        resources_clean: true,
    }
}

impl<A: VoiceSessionAdapters> VoiceSession<A> {
    pub fn new(generation: u64, adapters: A) -> Self {
        Self {
            generation,
            chord: Vec::new(),
            worker_alive: false,
            device_connected: false,
            voice_channel: false,
            audio_ready: false,
            winuhid_ready: false,
            pressed: false,
            first_audio_packet: false,
            injection: InjectionOutcome::NotAttempted,
            audio: AudioOutcome::Unknown,
            phase: VoicePhase::Idle,
            status_code: "not_ready".into(),
            detail: "等待 RC003 连接和语音环境就绪".into(),
            primary_action: PrimaryAction::ConnectRemote,
            resources_clean: true,
            adapters,
        }
    }

    pub fn generation(&self) -> u64 {
        self.generation
    }

    pub fn adapters(&self) -> &A {
        &self.adapters
    }

    pub fn adapters_mut(&mut self) -> &mut A {
        &mut self.adapters
    }

    /// Handle an event from the current BLE generation.
    pub fn handle(&mut self, event: VoiceEvent) -> VoiceSnapshot {
        self.handle_from(self.generation, event)
    }

    /// Handle an event and reject callbacks from an older generation.
    pub fn handle_from(&mut self, generation: u64, event: VoiceEvent) -> VoiceSnapshot {
        if generation != self.generation {
            // A late callback is observational only: it must not overwrite the
            // current session's readiness or cleanup state.
            return self.snapshot();
        }

        match event {
            VoiceEvent::ObserveReadiness {
                worker_alive,
                device_connected,
                voice_channel,
                audio_ready,
                winuhid_ready,
            } => {
                self.worker_alive = worker_alive;
                self.device_connected = device_connected;
                self.voice_channel = voice_channel;
                self.audio_ready = audio_ready;
                self.winuhid_ready = winuhid_ready;
                self.recompute_readiness();
            }
            VoiceEvent::Press { chord } => self.press(chord),
            VoiceEvent::Release => self.release(),
            VoiceEvent::AudioFrame(frame) => self.audio_frame(&frame),
            VoiceEvent::Disconnect => self.disconnect(),
            VoiceEvent::Shutdown => self.shutdown(),
        }
        self.snapshot()
    }

    fn press(&mut self, chord: Vec<u16>) {
        if self.pressed {
            return;
        }
        if chord.is_empty() {
            self.injection = InjectionOutcome::Failed;
            self.status_code = "empty_shortcut".into();
            self.detail = "语音键没有有效快捷键".into();
            return;
        }
        // The audio client may be cold and still be prepared on the first
        // press. WinUHid, the BLE channel, and the worker are the hard gates;
        // a failed prepare is reported without injecting the hotkey.
        if !(self.worker_alive && self.device_connected && self.voice_channel && self.winuhid_ready)
        {
            self.injection = InjectionOutcome::Failed;
            self.status_code = "voice_not_ready".into();
            self.detail = self.missing_readiness_detail();
            return;
        }

        if let Err(error) = self.adapters.prepare_audio() {
            self.fail_press("audio_prepare_failed", &error);
            return;
        }
        self.audio_ready = true;
        if let Err(error) = self.adapters.press_hotkey(&chord) {
            let _ = self.adapters.release_all();
            let _ = self.adapters.end_audio();
            self.chord.clear();
            self.fail_press("hotkey_press_failed", &error);
            return;
        }
        if let Err(error) = self.adapters.clear_audio() {
            let _ = self.adapters.release_hotkey(&chord);
            let _ = self.adapters.end_audio();
            self.chord.clear();
            self.fail_press("audio_clear_failed", &error);
            return;
        }

        self.chord = chord;
        self.pressed = true;
        self.first_audio_packet = false;
        self.injection = InjectionOutcome::Ready;
        self.audio = AudioOutcome::Ready;
        self.phase = VoicePhase::Pressed;
        self.status_code = "voice_pressed".into();
        self.detail = "已按下语音键".into();
        self.primary_action = PrimaryAction::None;
        self.resources_clean = false;
    }

    fn release(&mut self) {
        if !self.pressed {
            return;
        }
        let chord = self.chord.clone();
        let release_result = self.adapters.release_hotkey(&chord);
        let end_result = self.adapters.end_audio();
        let release_ok = release_result.is_ok();
        let end_ok = end_result.is_ok();
        let release_error = release_result.err();
        let end_error = end_result.err();
        self.pressed = false;
        self.chord.clear();
        if !end_ok {
            self.audio_ready = false;
        }
        self.injection = if release_ok {
            InjectionOutcome::Released
        } else {
            InjectionOutcome::Failed
        };
        self.audio = if end_ok {
            AudioOutcome::Stopped
        } else {
            AudioOutcome::Failed
        };
        self.phase = if self.voice_ready() {
            VoicePhase::Ready
        } else {
            VoicePhase::Recovering
        };
        self.status_code = if release_ok && end_ok {
            "voice_released".into()
        } else {
            "voice_release_failed".into()
        };
        self.detail = if release_ok && end_ok {
            "已松开语音键".into()
        } else {
            let detail = release_error
                .or(end_error)
                .unwrap_or_else(|| "未知释放错误".into());
            format!("语音资源释放失败：{detail}")
        };
        self.primary_action = PrimaryAction::None;
        self.resources_clean = release_ok && end_ok;
    }

    fn audio_frame(&mut self, frame: &[u8]) {
        if !self.pressed {
            self.audio = AudioOutcome::Failed;
            self.status_code = "audio_without_press".into();
            self.detail = "未按住语音键时收到音频".into();
            return;
        }
        match self.adapters.push_audio(frame) {
            Ok(()) if !self.first_audio_packet => {
                self.first_audio_packet = true;
                self.audio = AudioOutcome::FirstPacket;
                self.status_code = "first_audio_packet".into();
                self.detail = "已收到首个音频包".into();
            }
            Ok(()) => {
                self.audio = AudioOutcome::FirstPacket;
            }
            Err(error) => {
                self.audio = AudioOutcome::Failed;
                self.status_code = "audio_push_failed".into();
                self.detail = format!("音频发送失败：{error}");
                self.phase = VoicePhase::Recovering;
                self.primary_action = PrimaryAction::RepairAudio;
            }
        }
    }

    fn disconnect(&mut self) {
        self.cleanup_resources();
        self.worker_alive = false;
        self.device_connected = false;
        self.voice_channel = false;
        self.audio_ready = false;
        self.pressed = false;
        self.phase = VoicePhase::Disconnected;
        self.status_code = "disconnected".into();
        self.detail = "RC003 已断开".into();
        self.primary_action = PrimaryAction::ConnectRemote;
        self.recompute_primary_action();
    }

    fn shutdown(&mut self) {
        self.cleanup_resources();
        self.worker_alive = false;
        self.device_connected = false;
        self.voice_channel = false;
        self.audio_ready = false;
        self.winuhid_ready = false;
        self.pressed = false;
        self.phase = VoicePhase::Stopped;
        self.status_code = "stopped".into();
        self.detail = "语音会话已停止".into();
        self.primary_action = PrimaryAction::None;
    }

    fn cleanup_resources(&mut self) {
        let released = self.adapters.release_all();
        let ended = self.adapters.end_audio();
        self.chord.clear();
        self.resources_clean = released.is_ok() && ended.is_ok();
    }

    fn fail_press(&mut self, code: &str, detail: &str) {
        self.injection = InjectionOutcome::Failed;
        self.audio = AudioOutcome::Failed;
        self.phase = VoicePhase::Recovering;
        self.status_code = code.into();
        self.detail = detail.to_string();
        self.primary_action = PrimaryAction::Retry;
        self.resources_clean = true;
    }

    fn voice_ready(&self) -> bool {
        self.worker_alive
            && self.device_connected
            && self.voice_channel
            && self.audio_ready
            && self.winuhid_ready
    }

    fn missing_readiness_detail(&self) -> String {
        match (
            self.worker_alive,
            self.device_connected,
            self.voice_channel,
            self.audio_ready,
            self.winuhid_ready,
        ) {
            (false, _, _, _, _) => "语音 worker 尚未运行".into(),
            (_, false, _, _, _) => "RC003 尚未连接".into(),
            (_, _, false, _, _) => "ATVV 语音通道尚未就绪".into(),
            (_, _, _, false, _) => "音频路由尚未就绪".into(),
            (_, _, _, _, false) => "WinUHid 尚未就绪".into(),
            _ => "语音环境尚未就绪".into(),
        }
    }

    fn recompute_readiness(&mut self) {
        self.phase = if self.voice_ready() {
            VoicePhase::Ready
        } else if self.device_connected {
            VoicePhase::Recovering
        } else {
            VoicePhase::Connecting
        };
        if self.voice_ready() {
            self.status_code = "ready".into();
            self.detail = "语音环境已就绪".into();
            self.primary_action = PrimaryAction::None;
        } else {
            self.status_code = "not_ready".into();
            self.detail = self.missing_readiness_detail();
            self.recompute_primary_action();
        }
    }

    fn recompute_primary_action(&mut self) {
        self.primary_action = if !self.worker_alive || !self.device_connected {
            PrimaryAction::ConnectRemote
        } else if !self.voice_channel {
            PrimaryAction::Retry
        } else if !self.winuhid_ready {
            PrimaryAction::RepairWinUHid
        } else if !self.audio_ready {
            PrimaryAction::RepairAudio
        } else {
            PrimaryAction::Retry
        };
    }

    pub fn snapshot(&self) -> VoiceSnapshot {
        VoiceSnapshot {
            generation: self.generation,
            worker_alive: self.worker_alive,
            device_connected: self.device_connected,
            voice_channel: self.voice_channel,
            voice_ready: self.voice_ready(),
            first_audio_packet: self.first_audio_packet,
            pressed: self.pressed,
            injection: self.injection,
            audio: self.audio,
            phase: self.phase,
            status_code: self.status_code.clone(),
            detail: self.detail.clone(),
            primary_action: self.primary_action,
            resources_clean: self.resources_clean,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::rc::Rc;

    struct FakeAdapters {
        calls: Rc<RefCell<Vec<String>>>,
        fail_push: bool,
    }

    impl FakeAdapters {
        fn new() -> Self {
            Self {
                calls: Rc::new(RefCell::new(Vec::new())),
                fail_push: false,
            }
        }
    }

    impl VoiceSessionAdapters for FakeAdapters {
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
            if self.fail_push {
                Err("fake audio failure".into())
            } else {
                Ok(())
            }
        }
        fn release_all(&mut self) -> Result<(), String> {
            self.calls.borrow_mut().push("release_all".into());
            Ok(())
        }
    }

    fn ready_session() -> (VoiceSession<FakeAdapters>, Rc<RefCell<Vec<String>>>) {
        let adapters = FakeAdapters::new();
        let calls = Rc::clone(&adapters.calls);
        let mut session = VoiceSession::new(7, adapters);
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
    fn worker_and_device_alone_are_not_voice_readiness() {
        let snapshot = snapshot_from_readiness(1, true, true, false, true, true);
        assert!(!snapshot.voice_ready);
        assert_eq!(snapshot.status_code, "voice_channel_missing");
    }

    #[test]
    fn press_orders_prepare_then_hotkey_then_clear() {
        let (mut session, calls) = ready_session();
        let snapshot = session.handle(VoiceEvent::press(vec![0xA2, 0x44]));
        assert!(snapshot.voice_ready);
        assert!(snapshot.pressed);
        assert_eq!(
            calls.borrow().clone(),
            vec!["prepare", "down:[162, 68]", "clear"]
        );
    }

    #[test]
    fn first_packet_is_separate_from_readiness() {
        let (mut session, _calls) = ready_session();
        let before = session.handle(VoiceEvent::press(vec![0xA2, 0x44]));
        assert!(!before.first_audio_packet);
        let after = session.handle(VoiceEvent::AudioFrame(vec![1, 2, 3]));
        assert!(after.first_audio_packet);
        assert_eq!(after.audio, AudioOutcome::FirstPacket);
    }

    #[test]
    fn missing_winuhid_fails_closed_without_calling_hotkey() {
        let adapters = FakeAdapters::new();
        let calls = Rc::clone(&adapters.calls);
        let mut session = VoiceSession::new(1, adapters);
        session.handle(VoiceEvent::ObserveReadiness {
            worker_alive: true,
            device_connected: true,
            voice_channel: true,
            audio_ready: true,
            winuhid_ready: false,
        });
        let snapshot = session.handle(VoiceEvent::press(vec![0xA5]));
        assert!(!snapshot.voice_ready);
        assert_eq!(snapshot.injection, InjectionOutcome::Failed);
        assert!(calls.borrow().is_empty());
    }

    #[test]
    fn release_and_disconnect_clean_resources() {
        let (mut session, calls) = ready_session();
        session.handle(VoiceEvent::press(vec![0xA5]));
        session.handle(VoiceEvent::Release);
        session.handle(VoiceEvent::Disconnect);
        let snapshot = session.snapshot();
        assert!(snapshot.resources_clean);
        assert!(!snapshot.pressed);
        assert!(calls.borrow().iter().any(|c| c == "release_all"));
    }

    #[test]
    fn stale_generation_is_ignored() {
        let (mut session, _calls) = ready_session();
        let before = session.snapshot();
        let after = session.handle_from(6, VoiceEvent::press(vec![0xA5]));
        assert_eq!(after, before);
    }
}
