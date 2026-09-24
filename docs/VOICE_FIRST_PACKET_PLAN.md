# First audio packet

The production press path is owned by `VoiceSession`:

1. The session prepares the audio route.
2. It injects the configured RC003 hotkey down.
3. It clears the router session.
4. Decoded audio frames enter the same session.
5. The first successful frame is recorded separately from `voice_ready`.

The old static step list is not a production mechanism. `voice_pcm` warmup is single-flight and generation-cancellable; a failed or stopped warmup cannot install a late client.
