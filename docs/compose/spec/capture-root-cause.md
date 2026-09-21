---
feature: capture-root-cause
status: in-progress
updated: 2026-09-21
branch: debug/capture-root-cause
commits:
---

# Capture Root Cause Diagnostics

## Report

## [S1] Problem

用户映射热键自动录入时按键通常录不上。第一轮插桩证明：start 仅 11ms（排除短竞态），物理键在 swallow/engine 就绪期间 **零 swallow、零 publish**。第二轮证明：**零 hook_proc**——物理键从未进入 LL 钩子回调。

剩余分叉：

1. **假就绪**：Windows 静默卸钩，`is_hook_armed` 仍看 HOOK_PTR≠空。
2. **外钩抢占**：更早安装的 LL 钩子吞键且不 CallNext。

## [S2] Design

诊断插桩 + 单次可判读的强制实验（同一安装包一次验完）。

### 已有观测（第一轮）

start 耗时、cancel 三态、swallow/engine/capturing、publish、drain、hook start/armed/exit。

### 第二轮已证实

录入窗口内 **0 条 hook_proc** → 键没进回调，不是引擎/前端问题。

### 第三轮（本包）一揽子验证

| 改动 | 判读 |
|------|------|
| `ensure_hook_for_capture` 每次录入 `bump_hook_to_front` 强制重装 | 重装后 probe/物理键恢复 → **假就绪** |
| `hook_proc` 入口打点 + `note_hook_proc_hit` | 有 hook_proc 但 injected → 放行问题 |
| 启动后注入 **F24 探针**，120ms 内看钩子是否收到 | `probe_seen=false` 且重装后仍无 → **外钩吃键/钩子仍死**；`probe_seen=true` → 钩子活着，再看用户物理键 hook_proc | 

日志前缀均为 `[DEBUG-cap]`。

### 判读决策树（安装后只测一轮）

1. `probe seen=true` + 用户键有 `hook_proc` + 有 `publish` → **已修复（假就绪，强制重装有效）**
2. `probe seen=true` + 用户键有 `hook_proc` 但 injected 或无 publish → 看 injected/引擎
3. `probe seen=false` → 重装后仍收不到 → **外钩或 SetWindowsHookEx 实际失败**
4. `probe seen=true` + 用户键仍无 `hook_proc` → 物理键被更前置钩子吃掉（与 F1 冲突软件一致）

### CI

`build.yml`：`debug/*` 触发 build-nsis。

## [S3] Out of Scope

- 不改前端 Vue、不发 Release、不动 main。
- 不在本机编 Rust（用户禁令）；一律 CI。

## Tasks

- [x] T1: `[DEBUG-cap]` 插桩 — 第一/二轮已出包并取证 (covers: S2)
- [x] T2: `build.yml` debug/* — 已生效 (covers: S2)
- [x] T3: 第三轮：强制 bump + F24 探针 + hook_proc 命中 — 一次 push CI (covers: S2)
