# Voice VibeCoding

This context describes the Windows voice-remote product: turning a Xiaomi RC003 remote into reliable computer button input and press-to-talk voice input.

## Language

**RC003**:
The Xiaomi Remote 2 Pro hardware supported by this product context.
_Avoid_: T1, V60, generic remote support

**Button mapping**:
The association between one physical RC003 button and the computer action it produces.
_Avoid_: key configuration, shortcut editor

**Button capture**:
The active process of collecting a physical key sequence as a candidate Button mapping. Every observed key, including Esc, is a candidate during capture.
_Avoid_: implicit cancel, Escape-to-cancel

**Capture cancellation**:
The explicit end of a Button capture without accepting a candidate. It is a separate user action or lifecycle event, never inferred from a captured key.
_Avoid_: Escape-to-cancel, accidental capture commit

**Press-to-talk voice**:
A voice interaction that begins when the RC003 voice button is pressed and ends when it is released.
_Avoid_: toggle voice, hands-free voice

**Automatic gain**:
The bounded live adjustment of voice input toward a target output level so distant speech remains audible. The user-facing outcome is intelligible speech, not a particular displayed gain number.
_Avoid_: fixed volume boost, automatic microphone permission

**Voice readiness**:
The condition in which a press-to-talk attempt has all required user-visible prerequisites available, including pairing, remote connection, voice channel, virtual input, and audio delivery.
_Avoid_: bridge alive, worker alive, merely connected

**First audio packet**:
The first audio sample delivered toward the computer audio route after a press-to-talk begins.
_Avoid_: first log line, first UI update

**External setup handoff**:
When a required system or vendor condition is currently absent, the application reports the concrete action the user must take outside the product and offers a normal explicit repair action. The application checks current state rather than inferring whether the environment changed, and it does not run an automatic retry loop.
_Avoid_: historical eligibility guess, persistent repair center, automatic recovery

**Input-method setup**:
The configuration a user performs in an external input method so that its voice shortcut matches the RC003 mapping. The product does not own this setup, does not depend on a particular input method, and does not ship executable input-method shortcut presets.
_Avoid_: in-app mapping alone, input-method binding, built-in IME preset

**Program-managed capability**:
A capability the application owns end to end and can observe, retry, and clean up, such as RC003 connection state, input routing, and audio delivery.
_Avoid_: external prerequisite, vendor setup

**External prerequisite**:
A system, input-method, or vendor-owned condition that the application can detect and explain but cannot safely treat as its own state, such as Bluetooth pairing, driver installation, UAC, reboot, and input-method configuration.
_Avoid_: automatic repair, app-owned setup

**Guided installation**:
A flow where the application explains an external setup, launches the authoritative installer or script, waits for it, and verifies the resulting state. Explicit confirmation is required for an interactive or system-changing flow; a bounded embedded setup with no interface may begin automatically.
_Avoid_: success-by-exit-code, unexplained system mutation

**Current-state check**:
A direct observation of whether a required capability exists now; the application does not infer eligibility from a previous failure.
_Avoid_: historical eligibility guess, recovery scoring

**CABLE Output**:
The Windows recording endpoint exposed by VB-CABLE and selected by the user's external input method; it is not made the system default by this product.
_Avoid_: default microphone, input-method binding
