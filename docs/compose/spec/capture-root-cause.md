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

热键通常录不上。取证：物理键可录时 F24 探针仍恒 `seen=false`（SendInput 假阴性）。硬门闩导致「点录入直接报错、无法进入录制」。

## [S2] Design

1. **探针不否决 start**：失败只做 `bump_and_settle` 恢复 + warn，仍 `Ok` 进入录入。
2. **硬失败仅 `armed=false`**（钩子线程/句柄都没起来）。
3. **运行时健康**：WebView keydown = 漏键 → 第 1 次 recovery，≥2 次 `health_failed` → UI 红字退出。
4. **F24 旁路**：不进 engine/blocked；补 MapVirtualKey 扫描码提高探针灵敏度。
5. **overlap bump + hook_bump settle**；ensure 不再每次盲 bump。
6. **UI**：`capturing=true` 仅在 start 成功之后。
7. 契约测试 `capture_ready_contract.rs`。

### 判读

| 现象 | 含义 |
|------|------|
| start Ok + 能录 | 正常 |
| start Err（armed） | 钩子线程未起来 |
| start Ok 但立刻 health 红字 | 物理键漏进 WebView |
| probe_seen=false 仍能录 | 探针假阴性，已知 |

## [S3] Out of Scope

- 不整仓搬上游 F5；不发 Release；本机不装 Rust

## Tasks

- [x] T1–T2 插桩 / debug CI
- [x] T3 门闩+旁路+overlap+leak（初版硬门闩）
- [x] T4 探针改软门闩 + 扫描码（修「无法录入」误伤）
