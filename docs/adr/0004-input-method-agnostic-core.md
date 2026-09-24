# The voice core is input-method agnostic

The product sends a user-configured hotkey and delivers the remote microphone audio; it does not depend on a particular input method for its runtime behavior. Doubao and Qianwen are primary real-device acceptance cases, not special runtime modes. Executable input-method shortcut presets and in-app input-method setup are removed; users configure the external input method themselves, while the product keeps generic hotkey mapping and verification.
