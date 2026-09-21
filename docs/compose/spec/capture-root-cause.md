---
feature: capture-root-cause
status: designed
updated: 2026-09-21
branch: debug/capture-root-cause
commits:
---

# Capture Root Cause Diagnostics

## Report

## [S1] Problem

用户在映射热键「自动录入」时，按键后通常录不上。代码审查发现两条丢键路径，但安装包日志被噪音表滤掉，无法在本机证明实际走的是哪一条：

1. **短竞态**：前端先 `capturing=true`，后端 `start()` 最后才 `SWALLOW_ACTIVE=true`；窗口内按键被 WebView `preventDefault` 吞掉且引擎收不到。
2. **钩子假就绪**：Windows 可静默卸掉 `WH_KEYBOARD_LL`，而 `is_hook_armed` 只看 `HOOK_PTR != null`，仍报 armed；`start()` 成功但无钩子收键。

缺少运行时证据：失败当次的 swallow/engine/capturing、钩子 start/exit、start 耗时。

## [S2] Design

纯诊断插桩，不改录入语义、不修 bug。所有新日志前缀 `[DEBUG-cap]`，避开 `logging.rs` 噪音表（`Shortcut capture` / `Shortcut captured` / `SPECIAL KEY` 等）。

### 观测点

| 点 | 位置 | 区分的假设 |
|----|------|------------|
| start 进入/结束耗时 | `ShortcutCaptureSession::start` | 竞态窗口有多长 |
| start 时 armed/running | 同上 | 钩子是否可用 |
| cancel 进入时三态 | `cancel` | 排空/白吞 |
| 每次吞键：submitted/engine/capturing | `try_swallow_capture_key` | 钩子没收到 vs 白吞 vs 引擎未就绪 |
| publish 实际 keys | `publish_result` | 是否录到但 UI 未更新 |
| drain 分支 | `maybe_finish_hook_after_drain` | 吞键何时关 |
| 钩子 start/armed/exit | `special_keys` | 静默卸钩 / 线程退出 |

### 与候选假设的判读

- **按键后无任何 `[DEBUG-cap] swallow`** → 钩子未收到（假就绪或未 armed）→ 候选 2。
- **有 swallow 但 `engine=false` 或 `capturing=false`** → 白吞/状态错位。
- **有 swallow 且 engine/capturing=true，随后无 publish** → 引擎未识别（键型/顺序）。
- **有 publish，前端仍显示未录入** → UI/事件路径问题（已有 poll 兜底则更偏 emit 丢失）。
- **start 耗时 ≫ IPC 正常值（数百 ms～1.6s）且用户在返回前按键** → 候选 1。

### CI

`build.yml` 的 `push.branches` 增加 `debug/*`，以便推送 `debug/capture-root-cause` 触发 `build-nsis` 出安装包。逻辑零改动（仅 workflow 触发条件）。

## [S3] Out of Scope

- 不修竞态、不修 `is_hook_armed`、不改噪音过滤策略（保留过滤，依赖 `[DEBUG-cap]` 前缀逃逸）。
- 不改前端 Vue。
- 不发 Release / 不动 main。

## Tasks

- [ ] T1: 在 `shortcut_capture.rs` / `special_keys.rs` 加 `[DEBUG-cap]` 插桩 — acceptance: 启动/录入路径产生可区分候选 1/2 的日志行，且不含行为变更 (covers: S2)
- [ ] T2: `build.yml` 支持 `debug/*` 分支触发 — acceptance: push 到 `debug/capture-root-cause` 会跑 build-nsis (covers: S2)
- [ ] T3: 推送分支并确认 CI success，产出 NSIS artifact — acceptance: Actions run conclusion=success 且有 Voice-VibeCoding-NSIS (covers: S2)
