# Voice readiness is a strict product state

The product reports voice readiness only when the RC003 connection and all required voice-input prerequisites are available. A running worker or an open process is not sufficient. The first successful audio packet is tracked separately as verification of an actual press-to-talk attempt.
