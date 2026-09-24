# Hold-to-talk voice

RC003 voice interaction is hold-only:

- Press: inject the configured generic hotkey and start the audio session.
- Release: release the hotkey and end the audio session.
- Disconnect, restart, or shutdown: release the chord and stop audio resources.

The application does not configure an external input method. It does not modify the Windows default microphone or microphone privacy settings. Select `CABLE Output` in the external input method.

The production seam is `VoiceSession::handle(event) -> VoiceSnapshot`; readiness and first audio packet remain separate facts.
