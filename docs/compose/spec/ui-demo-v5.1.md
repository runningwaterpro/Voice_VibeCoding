---
feature: ui-demo-v5.1
status: delivered
updated: 2026-09-18
branch: feat/ui-demo-v5.1
commits: 61d4729..0e9bdea
---

# UI Demo v5.1（状态分层 · 映射卡精简 · 修复成功不挡）

## Report

**What was built** — 新增 `docs/design/ui-redesign-demo-v5.1.html`（不改 v5.0）：状态唯一主操作分层（异常条 ⊕ 居中卡 ⊕ 健康态皆无）、映射卡默认无三键仅选中展开并显示 `已绑定 n/m`、`done_ok` 强制不弹修复模态。附 `scripts/verify-ui-demo-v51.mjs` 静态验收。

**Verification** — `node scripts/verify-ui-demo-v51.mjs` → ALL PASS（23 项）；`git diff 61d4729 -- ui-redesign-demo-v5.0.html` 空；Review：T1–T5 全 Met、无 critical。

**Journey log** — 本地无 `main`，worktree 基线用 `gh/main`；v5.0 在工作区未跟踪副本与 `gh/main` 标题文案可能不一致，以 worktree 内 tracked 文件为准；`data-force-repair` 无 handler 为 v5.0 预存问题，本 feature 不修。

## [S1] Problem

在 `ui-redesign-demo-v5.0.html` 样式预览上，用户看实际效果前只批准三项体验改动：

1. 同一异常态可能同时看到状态栏文案、异常条主按钮、居中修复卡，主操作不唯一。  
2. 映射卡默认铺开「录入 / 手动组合 / 清除」+ 说明句，列表噪、纵向占位大；无已绑定计数。  
3. 一键修复全成功（`done_ok`）仍弹居中模态，需手动关闭，挡主界面。

必须新建 `ui-redesign-demo-v5.1.html`，不得覆盖或改写 `v5.0`。

## [S2] Design

工作区：`.worktrees/ui-demo-v51` · 分支 `feat/ui-demo-v5.1` · 基线 `61d4729`。

产物：`docs/design/ui-redesign-demo-v5.1.html`（由 v5.0 复制后修改）。v5.0 只读。

### 1. 状态唯一主操作（item 1）

按 `data-state` / `STATES[name]` 统一 `applyState` 可见性：

| 状态类 | 异常条 `#actionBar` | 居中卡 `#repairModal` | 主操作 |
|---|---|---|---|
| `ready` / `voice` / `done_ok` | 隐藏 | **隐藏**（见 §3） | 无（仅状态栏） |
| `error` / `err_*` | **显示** | 隐藏 | 异常条 `#btnPrimary`（或 secondary） |
| `booting` / `repairing` | **隐藏** | **显示** | 修复卡内按钮（repairing 可无按钮） |
| `done_partial` / `done_fail` | 隐藏 | **显示** | 修复卡主按钮 |
| `idle` / `connecting` / `err_bridge` | **显示** | 隐藏 | 异常条主按钮 |

合同：

- 任一预览态下，`.btn.attention` / `.btn.primary` 作为「用户第一眼主操作」**至多一处**可点击高亮。  
- 状态栏 LED + `#headline` 始终保留作环境状态，不单独构成第二主按钮。  
- `done_ok`：无异常条、无修复模态（§3）。  
- demo-bar 切换状态时必须跑同一 `applyState`，禁止残留另一层主操作。

### 2. 映射卡默认精简 + 已绑定计数（item 2）

`renderMaps()` 输出：

- 默认卡体：键名 + 绑定文案一行区；**不渲染** `.map-card-actions`（录入/手动组合/清除）与 `.capture-live` 提示句。  
- `selectedKey === k.id` 或 `capturing && selectedKey === k.id` 时才渲染 actions（录入中保留 `请按目标键…`）。  
- `hover` 仅加边框高亮，不展开按钮（避免 hover 抖列表）。  
- 映射区标题或 `#mapWork` 顶部增加计数：`已绑定 {bound}/{total}`，`bind !== "未映射"` 计 bound；`ghost` 不计 total。  
- 计数节点 id：`#bindCount`，随 `renderMaps` 更新。  
- 点击卡：设置 `selectedKey` 并 `renderMaps()`；点已选中卡空白处可取消选中（可选，不强制）。

### 3. 修复全成功不挡（item 5）

- `STATES.done_ok`（及 `applyState("done_ok")`）：`repairModal` 保持 `hidden`；不依赖 `showRepairModal`。  
- `repairing` / `done_partial` / `done_fail` / `booting`：仍按现逻辑显示模态。  
- 成功反馈仅靠状态栏（`语音可用` + 绿 LED）与 demo-bar 可见的状态切换。  
- 若 `done_ok` 误带 `repairSteps`，不渲染进 DOM（无模态载体）。

### 4. 标识

- `<title>` 与 `.demo-bar` 文案含 `v5.1`，避免与 5.0 混淆。  
- 不修改 `src/` 产品代码。

## [S3] Out of Scope

- 电平折叠（item 3）、空态引导计数文案扩展（item 4 全文）、设置页增强。  
- 改写 `ui-redesign-demo-v5.0.html` 或删除旧 demo。  
- 产品 `src/App.vue` / `SideNav` 等运行时代码。  
- 配色、遥控器示意图重绘、左侧设备 IA。

## Tasks

- [x] T1: 复制 v5.0 → `ui-redesign-demo-v5.1.html` 并改 title/demo-bar 标识 — 验收: 文件存在；打开标题为 v5.1；`ui-redesign-demo-v5.0.html` git 状态未变 (covers: S2.4)
- [x] T2: 实现状态唯一主操作分层 — 验收: 对 `ready`/`error`/`booting`/`repairing`/`done_ok`/`done_partial` 切换后，主操作仅存在于表中规定的一层 (covers: S2.1; depends: T1)
- [x] T3: 映射卡默认无三键 + `#bindCount` — 验收: 未选中卡无 `.map-card-actions`；存在 `已绑定 n/m`；选中卡恢复按钮 (covers: S2.2; depends: T1)
- [x] T4: `done_ok` 不显示修复模态 — 验收: 点 demo-bar「全部成功」后 `#repairModal` 为 hidden，无异常条 (covers: S2.1, S2.3; depends: T2)
- [x] T5: 预览点检 — 验收: 脚本 `scripts/verify-ui-demo-v51.mjs` ALL PASS；人工打开 HTML 看样式 (covers: S2; depends: T2, T3, T4)
