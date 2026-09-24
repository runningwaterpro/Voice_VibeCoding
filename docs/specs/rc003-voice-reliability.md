## Problem Statement

当前 RC003 语音链路虽然已经具备 BLE、HID、ATVV、VB-CABLE、WinUHid 和音频路由等实现，但用户无法得到稳定、可信的反馈：后台 worker 活着可能被误认为语音可用，连接、重连、修复和 UI 状态由多处逻辑分别判断；WinUHid 依赖可能失败且错误反馈不直观；程序还会修改 Windows 默认麦克风并影响其他程序；界面仍保留 T1/V60 和输入法专用快捷设置，增加认知负担和维护成本。

用户真正要完成的任务是：使用 RC003，在外部输入法的语音快捷键已经配置好的前提下，按住语音键说话，松开结束，并确信程序不会留下残留按键、后台线程或错误的系统状态。

## Solution

将产品收敛为 RC003-only、hold-only、输入法无关的语音热键与音频传输工具。

建立一个真实的 VoiceSession module 作为唯一主要生产 seam：它接收 press、release 和 audio frame 事件，隐藏 WinUHid、PCM、GATT 和 UI 细节，并返回当前 VoiceSnapshot。VoiceSnapshot 明确区分 worker 存活、设备连接、Voice readiness 和 first audio packet。

程序只根据当前真实状态工作：WinUHid 存在就显示已就绪，不存在就显示正常的安装/修复引导；不根据历史失败记录猜测环境变化，不运行自动无限重试。用户可以主动再次发起修复，程序随后重新检查当前状态。

WinUHid 的无界面安装可以自动开始并显示步骤；需要 UAC 时由 Windows 要求用户确认。VB-CABLE 继续使用官方安装器界面。程序只检测 CABLE Input/CABLE Output，不修改 Windows 默认麦克风或麦克风权限；用户在豆包、千问等外部输入法中自行选择 CABLE Output。

第一阶段先完成用户入口、录入安全、Raw Input fail-closed、配置保存和状态反馈等低风险 P0 修复，再建立 VoiceSession 和 session lifecycle 的可靠测试 seam。

## User Stories

1. As an RC003 user, I want the application to support only the remote I actually own, so that I do not see unfinished device options.
2. As an RC003 user, I want T1 and V60 removed from the product surface, so that navigation reflects the supported product.
3. As an existing user, I want old T1/V60 configuration files left untouched, so that migration does not silently delete data.
4. As an RC003 user, I want press-and-hold voice behavior, so that pressing begins speech and releasing ends it.
5. As an RC003 user, I want legacy Toggle and hands-free options removed from active choices, so that the interface does not promise behavior that is not supported.
6. As an RC003 user, I want to configure any hotkey I need, so that the product works with my external input method.
7. As a Doubao user, I want the application to send the configured hotkey without special Doubao runtime behavior, so that the product remains input-method agnostic.
8. As a Qianwen user, I want the same generic hotkey behavior as other input methods, so that the application does not branch on input-method names.
9. As a user, I want the program not to ship executable input-method shortcut presets, so that input-method configuration remains an external responsibility.
10. As a user, I want the application to report Voice readiness only when all required RC003 voice prerequisites are available, so that a green status means something.
11. As a user, I want worker state, device connection, voice readiness, and first audio packet to be distinct, so that I can understand what has actually happened.
12. As a user, I want the first audio packet to be verifiable after a press, so that the application does not claim success based only on process state.
13. As a user, I want RC003 reconnection to be automatic for program-owned capabilities, so that ordinary disconnects do not require manual repair.
14. As a user, I want a reconnect or restart operation to wait for the previous session to finish, so that old callbacks cannot corrupt a new session.
15. As a user, I want repeated start, stop, and restart actions to be idempotent, so that they cannot create duplicate workers.
16. As a user, I want application shutdown to release all voice chords, modifiers, hooks, sessions, and audio resources, so that no key remains stuck.
17. As a user, I want WinUHid to be checked from its current actual state, so that a missing capability is never hidden by stale state.
18. As a user, I want WinUHid installation to start automatically when it is missing and has no interactive installer, so that I do not need to understand the dependency.
19. As a user, I want to see each WinUHid installation step, so that I know what the program is doing.
20. As a user, I want Windows UAC to remain the explicit administrator confirmation, so that the program does not pretend to bypass system security.
21. As a user, I want a normal repair action whenever WinUHid is currently absent, so that I am not trapped in a dead-end error state.
22. As a user, I want the application not to run an automatic infinite installation loop, so that a failed setup does not repeatedly prompt or change my system.
23. As a user, I want to explicitly request another repair attempt when I choose to, so that I retain control without an automatic retry policy.
24. As a user, I want the application to re-check current state after every repair attempt, so that success is based on the device rather than an exit code.
25. As a user, I want a failed WinUHid attempt to show a plain-language reason, so that I know whether I need administrator access, a restart, or external troubleshooting.
26. As a user, I want logs to be secondary diagnostic information, so that I am not forced to understand application logs to solve a normal setup problem.
27. As a user, I want VB-CABLE installation to use the official installer interface, so that the vendor owns the interactive installation flow.
28. As a user, I want the application to verify VB-CABLE endpoints after the official installer exits, so that installer exit status is not mistaken for endpoint readiness.
29. As a user, I want the application not to change my Windows default microphone, so that other applications continue to use my chosen microphone.
30. As a user, I want the application not to modify microphone privacy settings, so that installing the voice tool does not change unrelated system permissions.
31. As a Doubao or Qianwen user, I want to select CABLE Output in my input method myself, so that device selection remains under my control.
32. As a user, I want ordinary RC003 button mappings to remain usable when voice-specific WinUHid setup is unavailable, so that one blocked voice prerequisite does not falsely disable unrelated mappings.
33. As a user, I want the interface to show one primary next action and optional details, so that repair does not present several competing controls.
34. As a user, I want the current missing capability to remain visible, so that I do not think the application is ready when it is not.
35. As a user, I want raw-input fallback disabled when it cannot identify the RC003, so that ordinary keyboard input cannot trigger remote mappings.
36. As a user, I want shortcut capture to cancel safely on Escape and inactivity, so that my keyboard is never left swallowed.
37. As a user, I want a failed mapping save to roll back the displayed value, so that the interface never shows a mapping that was not persisted.
38. As a developer, I want T1/V60 code and commands removed, so that unsupported paths do not remain in the release surface.
39. As a developer, I want one VoiceSession interface to test, so that tests exercise production sequencing instead of disconnected helper functions.
40. As a developer, I want fake GATT, WinUHid, and PCM adapters, so that lifecycle and first-packet behavior can be tested without hardware.
41. As a release maintainer, I want CI to run frontend and Rust tests, so that a build-only pipeline cannot ship a broken reliability path.
42. As a release maintainer, I want a real Windows/RC003/Doubao/Qianwen smoke test, so that hardware-only failures are discovered before release.

## Implementation Decisions

1. The product scope is RC003-only. T1 and V60 routes, runtime configuration, commands, UI, tests, and active documentation are removed. Existing user files are not automatically deleted.
2. Press-to-talk is hold-only. Legacy configuration fields may be parsed for migration but are not active user choices.
3. The voice core is input-method agnostic. Generic hotkey mapping and audio transport remain; executable input-method presets and in-app input-method setup are removed.
4. Voice readiness is a strict product state. Worker survival, device connection, voice readiness, and first audio packet are separate facts.
5. A VoiceSession module is the primary production seam. It accepts press, release, and audio-frame events and returns a VoiceSnapshot containing readiness, injection outcome, first-packet verification, and resource state.
6. The VoiceSession implementation owns decoder state, press/release ordering, first-packet handling, and chord release. WinUHid, PCM, GATT, and UI notification remain internal adapters.
7. Session and process lifecycle scopes are explicit. BLE reconnect preserves process-level resources; explicit restart waits for the prior session; shutdown stops every resource exactly once; stale callbacks cannot affect a newer session.
8. Current-state checks are direct observations. The application does not infer current eligibility from a previous failure.
9. A missing external prerequisite produces a normal explicit repair action. The application does not run an automatic infinite repair loop. A user may explicitly request another attempt, after which current state is checked again.
10. WinUHid has no interactive installer in the product path. Its embedded, verified setup may start automatically and report step progress. UAC remains the system confirmation.
11. VB-CABLE uses the official interactive installer. The application waits for it and verifies CABLE endpoints afterward.
12. The application verifies CABLE Input and CABLE Output but never sets CABLE Output as the Windows default microphone and never changes microphone privacy settings.
13. Raw Input fallback is disabled in production unless a real device-identity filter is available. It must never treat arbitrary keyboard input as RC003 input.
14. The primary status surface shows one current fact and one primary action; logs and detailed diagnostics are secondary.
15. Configuration and mapping changes are committed only after persistence succeeds; failed saves roll back the visible draft.
16. The implementation begins with P0 user-facing and safety fixes, then establishes lifecycle tests, then performs the VoiceSession and supporting module refactors.
17. The release surface must not expose T1/V60, and in-app update behavior must not point at an unrelated upstream source.

## Testing Decisions

1. The highest test seam is the VoiceSession interface. Tests exercise externally observable behavior, not private helper implementation details.
2. A fake GATT adapter supplies press, release, audio frames, disconnects, stale callbacks, and generation changes.
3. A fake WinUHid adapter records injected hotkey down/up calls, failures, and release-all behavior.
4. A fake PCM adapter records route readiness, first packet, clear/end ordering, failures, and cancellation.
5. VoiceSession tests cover cold press ordering, ready press, release ordering, duplicate events, missing WinUHid, first-packet success, first-packet failure, disconnect, stale-session callbacks, restart, and shutdown cleanup.
6. Current-state setup tests cover absent WinUHid, present WinUHid, installer success, installer failure, UAC cancellation, explicit user retry, and no automatic retry loop.
7. Mapping tests cover generic hotkeys, microphone/voice synchronization during migration, persistence failure rollback, and removal of executable input-method presets.
8. Readiness tests assert that worker state alone cannot produce Voice readiness and that late battery or status events cannot falsely mark a disconnected device connected.
9. Raw Input tests verify that arbitrary keyboard events are ignored or the production fallback is disabled.
10. Default-microphone tests verify that setup never changes the default capture endpoint or microphone privacy settings.
11. Hardware acceptance uses real Windows 10/11, RC003, Doubao, and Qianwen. It verifies pairing, press-to-talk, first audio packet, release, ordinary button mappings, disconnect/reconnect, and clean shutdown.
12. CI runs frontend unit tests, frontend build, Rust tests, Rust formatting checks, and Windows integration tests where the environment supports them. Source-string contract tests are not sufficient as the only protection.
13. Existing capture, voice chord, first-packet, and configuration tests are prior art and should be retained or rewritten to cross the VoiceSession seam rather than duplicate production logic.

## Out of Scope

1. T1 and V60 support or migration into active product behavior.
2. Executable WeChat, Doubao, Qianwen, or other input-method shortcut presets.
3. Automatic configuration of an external input method.
4. Changing the Windows default microphone or microphone privacy settings.
5. Restoring or rewriting historical Git commits.
6. Cross-platform support for the Windows-only RC003 path.
7. A full visual redesign unrelated to readiness, repair, and input-method-agnostic behavior.
8. A new application updater until the Fork release source, version, hash, and signature flow are defined.
9. Unbounded automatic retries or a persistent repair center.
10. A numeric first-packet latency target before a real hardware baseline is measured.
11. Local TCP/UDP protocol authentication as part of this first reliability slice; it requires a separate security decision and ticket.

## Further Notes

- The current code already contains partial implementations of the target concepts, but the production voice path is split across many modules. The specification intentionally defines the target behavior before choosing every internal file-level refactor.
- “Voice readiness” does not mean that a first audio packet has already been observed. First audio packet is separate verification of an actual press-to-talk attempt.
- A missing WinUHid is a current product state, not a permanent verdict. The user may explicitly request another repair attempt; the product then checks the current state again.
- The first implementation should preserve ordinary button mappings where possible, but must clearly report that voice-specific injection is unavailable when WinUHid is absent.
- The real-device smoke test is release-blocking even when pure unit tests pass.
- This specification is published as GitHub Issue #1: https://github.com/runningwaterpro/Voice_VibeCoding/issues/1. The local file remains the source draft for future edits.
