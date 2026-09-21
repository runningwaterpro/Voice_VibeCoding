---
feature: capture-root-cause
status: in-progress
updated: 2026-09-21
branch: debug/capture-root-cause
commits:
---

# Capture Root Cause → Ready Gate

## Report

## [S1] Problem

映射热键「通常录不上」。实测：物理键不进 LL 回调；`is_hook_armed` 句柄非空会假就绪；曾用盲 bump + 探针但 F24 可能进引擎、UI 可先亮。

## [S2] Design

**就绪 = 探针收到键**，不是 `HOOK_PTR`。

1. **F24 探针旁路**：`feed_capture_key` 对 `PROBE_VK` 直接 return，禁止 publish。
2. **门闩**：start 内 probe → 失败则 `bump_and_settle`（overlap+generation）→ 再 probe → 仍失败 `Err`，UI 不进 capturing。
3. **漏键交叉确认**：capturing 期间 WebView 收到 keydown → `note_web_leak` → 第 1 次 recovery bump，≥2 次 `health_failed` → poll 让 UI 红字退出。
4. **钩子基建（借上游）**：`hook_bump` generation；WM_BUMP **先 Set 后 Unhook**；`ensure` **不再**每次强制 bump。
5. **Blocked 集合**：spin `try_lock`，失败再短 `lock`；KeyUp 禁止因失败丢弃。
6. **源码契约测试** `tests/capture_ready_contract.rs`。

### 判读

| 现象 | 含义 |
|------|------|
| start Err + probe false | 钩子收不到键 |
| 能录 + 无 leak | 正常 |
| leak≥2 + health_failed | 启动后钩子被外钩抢/死 |

## [S3] Out of Scope

- 不整仓搬上游 F5/voice_dispatch
- 不发 Release / 不动 main
- 本机不装 Rust（CI）

## Tasks

- [x] T1: 插桩与取证（已完成多轮）
- [x] T2: debug/* CI
- [x] T3: 门闩+旁路+overlap+leak+契约测试（本包）
