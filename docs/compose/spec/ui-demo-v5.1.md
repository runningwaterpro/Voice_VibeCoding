---
feature: ui-demo-v5.1
status: in-progress
updated: 2026-09-18
branch: feat/ui-demo-v5.1
commits: 61d4729..0fd575f
---

# UI Demo v5.1（居中卡唯一交互面 · 映射精简 · 成功不挡）

## Report

**What was built** — 初版误将异常送回顶栏 action-bar（与 `beeca74` 相反）。纠偏后：居中悬浮修复卡为**唯一**交互面；action-bar 永久隐藏；映射卡默认无三键 + `已绑定 n/m`；`done_ok` 不弹卡。

**Verification** — （纠偏完成后回填）

**Journey log** — 演进：V4 顶栏/左栏异常条 → `beeca74` drop top action bar → 居中浮层；产品 `XiaomiSettings.vue` 与 v5.0 均「一律居中卡」。初版 item1 走反，已按用户确认 A+B+D 回滚。

## [S1] Problem

1. 现行合同（`beeca74`、产品 Vue、v5.0）：**启动/异常/修复结果一律居中悬浮卡，不占文档流；顶栏 action-bar 已弃用。**  
2. v5.1 初版把 `error`/`idle`/`err_*` 送回 action-bar —— 老路，主操作与历史演进冲突。  
3. 仍保留的真问题：映射卡默认三键过噪、无已绑定计数；`done_ok` 不应弹卡。

必须纠正 `ui-redesign-demo-v5.1.html`，不得改写 `v5.0`。

## [S2] Design

工作区：`.worktrees/ui-demo-v51` · 分支 `feat/ui-demo-v5.1` · 基线 `61d4729`。

### 1. 居中卡 = 唯一交互面（纠偏 A + 合同 B）

| 状态 | `#actionBar` | `#repairModal` | 卡内主操作 |
|---|---|---|---|
| `ready` / `voice` / `done_ok` | **永久隐藏** | **隐藏** | 无 |
| `booting` | **永久隐藏** | **显示** booting | **无按钮**（只读步骤文案） |
| `error` / `err_*` / `idle` / `err_bridge` / `connecting` | **永久隐藏** | **显示** | **一个**主钮（由 `s.primary`/默认一键修复映射文案）+ 条件「稍后」 |
| `repairing` | **永久隐藏** | **显示** + steps | 无钮或按 `showRepairButtons` |
| `done_partial` / `done_fail` | **永久隐藏** | **显示** + steps | 唯一重试/一键修复 |

合同：

- `#actionBar` **任何状态不显示**（`setActionBarVisible(bar,false)` 或等价）；禁止 `showBar` 分支。  
- 状态栏只保留 LED + 环境文案，不出现 `.btn.attention`/`.btn.primary`。  
- 降级态主操作 **有且仅有** 居中卡内一处。  
- 溯源：`beeca74`、`53f43db`、`XiaomiSettings.vue`「统一居中卡」。

### 2. 映射卡默认精简 + 已绑定计数（保留 D）

- 未选中卡不渲染 `.map-card-actions`；仅 `isSel` 输出三键与录入提示。  
- `#bindCount`：`已绑定 {bound}/{total}`；`ghost` 不计 total；`未映射` 不计 bound。  
- hover 只描边不展开。

### 3. 修复全成功不挡（保留 B）

- `forceNoBlock`：`ready` / `voice` / `done_ok` → 卡与条皆无。  
- 失败/部分失败仍居中卡 + 步骤 + 唯一主钮。

### 4. 标识

- title / demo-bar 含 `v5.1`；文案不宣称「异常走状态条」。  
- 不改 `src/` 与 `v5.0`。

## [S3] Out of Scope

- 半透明遮罩、电平折叠、设置增强、产品 `src/`、改 v5.0。

## Tasks

- [x] T1: 复制并标识 v5.1 — 验收: title/demo-bar v5.1；v5.0 未改 (covers: S2.4)
- [ ] T2: **纠偏** — action-bar 任意状态不显示；`error`/`idle`/`err_*` 改为居中卡 + 卡内主钮 — 验收: 层逻辑 `modal=true, bar=false` 覆盖全部非健康态；无 showBar (covers: S2.1; depends: T1)
- [x] T3: 映射精简 + bindCount — 验收: 未选中无 actions；已绑定 n/m (covers: S2.2; depends: T1)
- [x] T4: done_ok 不弹卡 — 验收: modal hidden (covers: S2.3; depends: T1)
- [ ] T5: 更新 `verify-ui-demo-v51.mjs` 并 ALL PASS — 验收: error/idle/err_* expect modal=true bar=false；bar 永 false (covers: S2.1; depends: T2)
- [ ] T6: 提交纠偏 — 验收: 新 commit；Spec 与实现一致 (covers: S2; depends: T2, T5)
