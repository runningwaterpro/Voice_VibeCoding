---
feature: capture-root-cause
status: delivered
updated: 2026-09-21
branch: debug/capture-root-cause
commits: 4f11191..bc00403
---

# Capture Root Cause → Ready Gate

## Report

**What was built** — 修复映射热键「通常录不上」：前端在录入期间直接用 WebView `keydown` 组成并弦并保存（Shift+F10 / A / Ctrl+P 不再依赖低级钩子是否收到键）。Esc 与修饰键组合现在同样作为候选按键；取消只由界面、超时或生命周期触发。后端保留 LL 钩子吞键与诊断：探针不再否决录入、stop/start 线程必须 join、退出只卸自己的 HHOOK、overlap bump + generation settle、契约测试防回归。

**Verification** — 用户实测 Shift+F10、A、Ctrl+P 均正常录入。CI：`debug/capture-root-cause` 多次 `build-nsis` success；本包 `bc00403` run 35589997219 success。

**Journey log**
- F24 SendInput 探针恒假阴，不能当就绪唯一条件。
- 探针失败驱动 restart + 旧线程退出 Unhook 全局句柄 → 拆掉新钩子；已改为 join + 所有权。
- 键能稳定到达 WebView → 录入主路径改到前端 keydown。

## [S1] Problem

映射热键自动录入通常失败：钩子假就绪、探针假阴、漏键误报、甚至无法进入录入态。

## [S2] Design

1. **主路径**：`chordFromEvent` + WebView keydown → `onCaptured`（不依赖 LL 收到键）。
2. **辅路径**：LL `try_swallow` 吞键 + engine publish（钩子可用时）。
3. **线程所有权**：stop/restart join；exit 只卸 `mine` + CAS 清 `HOOK_PTR`。
4. **bump**：先 Set 后 Unhook + `hook_bump` generation settle。
5. **UI**：`capturing=true` 仅在 `capture_shortcut_start` 成功之后。
6. **契约测试** `capture_ready_contract.rs`。

### 判读

| 现象 | 含义 |
|------|------|
| 能录入 | 主路径生效（WebView keydown） |
| probe_seen=false 仍能录 | 探针假阴性（已知） |
| 日志 armed=false 仍能录 | 钩子非 gating（WebView 主路径） |
| 断线重连不停钩 | stop 仅退出/托盘/重启 |
| 点录入不会绑成 F24 | 前端忽略探针 VK 0x87 |

## [S3] Out of Scope

- 不整仓搬上游 F5/voice_dispatch
- 本机不装 Rust（CI 出包）

## Tasks

- [x] T1: 插桩与取证 — acceptance: 区分竞态/假就绪/漏键 (covers: S2)
- [x] T2: debug/* CI — acceptance: push 即 build-nsis (covers: S2)
- [x] T3: 线程所有权 + 前端 keydown 主路径 — acceptance: 用户三键实测通过 (covers: S2)
