# RC003 release acceptance

This gate is manual and release-blocking. Run it on a real Windows 10 or Windows 11 machine with the RC003, VB-CABLE, WinUHid, Doubao, and Qianwen available.

CI uses `cargo check --all-targets` as the blocking Rust gate. The full Windows `cargo test` harness is available as `npm run test:rust:run`, but is not used to block installer packaging because Tauri dialog dependencies can make the test executable fail at process startup with `STATUS_ENTRYPOINT_NOT_FOUND` even when compilation succeeds.

## Before setup

- [ ] Record the current Windows default microphone.
- [ ] Record microphone privacy permission values.
- [ ] Pair the RC003 in Windows Bluetooth.
- [ ] Choose an arbitrary RC003 voice hotkey in the app.
- [ ] Configure the same hotkey in the external input method.

## Setup and ordinary controls

- [ ] WinUHid status is obtained from the current device state.
- [ ] A missing WinUHid capability offers one explicit repair action and never starts an infinite retry loop.
- [ ] CABLE Input and CABLE Output are both detected.
- [ ] The default microphone and microphone privacy values are unchanged.
- [ ] Select CABLE Output in the external input method.
- [ ] Ordinary RC003 buttons work while voice-specific WinUHid is unavailable.

## Voice acceptance

- [ ] Press and hold the RC003 voice button: the configured hotkey goes down and the first audio packet is observed.
- [ ] Keep holding: audio continues toward the selected input method.
- [ ] Release: the hotkey goes up, the audio session ends, and no key remains stuck.
- [ ] Repeat in Doubao.
- [ ] Repeat in Qianwen.

## Lifecycle acceptance

- [ ] Disconnect and reconnect the RC003; no duplicate worker or stale status appears.
- [ ] Explicitly restart the bridge; the old session finishes before the new one starts.
- [ ] Exit with the voice button held; no key, hook, GATT session, or audio client remains.

Record the Windows version, RC003 firmware, app commit, and pass/fail result with the release.
