---
feature: ui-demo-v5.1
status: in-progress
updated: 2026-09-18
branch: feat/ui-demo-v5.1
commits: 61d4729..77796b6
---

# UI Demo v5.1（对齐产品基线）

## Report

**What was built** — v5.1 以产品 `XiaomiSettings.vue` + `KeyMappingStage.vue` 为**唯一对照**收成基线：居中卡、固定「一键修复」、无顶栏异常条；映射卡 actions 常驻 DOM + CSS 折叠（与产品一致）。

**Verification** — （对齐完成后回填）

**Journey log** — 曾误用 V4 分项 `primaryMap`（修复 ATVV/声卡…）与 `bindCount`，均非产品现状；`beeca74` 起顶栏 action-bar 已弃。产品主钮恒为 `一键修复`/`修复中…`。

## [S1] Problem

Demo 与程序不一致，不能当基线：

1. 卡内主钮曾映射为分项修复文案；产品只有 **一键修复 / 修复中…**。  
2. Demo 曾加 `已绑定 n/m`、默认不渲染 map actions；产品 actions **始终在 DOM**，靠 `.active` CSS 展开，**无**绑定计数。  
3. 异常文案应是 `errorTitle` +「点「一键修复」自动处理」，不是「点修复 ATVV…」。

目标：`ui-redesign-demo-v5.1.html` 与现行程序修复/映射交互**行为一致**，作后续优化前的 baseline。

## [S2] Design

工作区：`.worktrees/ui-demo-v51` · 分支 `feat/ui-demo-v5.1` · 基线 `61d4729`。

对照：`src/views/XiaomiSettings.vue`、`src/components/KeyMappingStage.vue`。不改 `src/`、不改 `v5.0`。

### 1. 居中卡 = 唯一交互面（与产品一致）

| 状态 | `#actionBar` | `#repairModal` | 卡内 |
|---|---|---|---|
| `ready` / `voice` / `done_ok` | 永久隐藏 | 隐藏（`allReady`） | — |
| `booting` | 永久隐藏 | 显示 booting | **无按钮**；文案 = 启动步骤句 |
| 其它非健康态 | 永久隐藏 | 显示 | 标题=`errorTitle` 或修复结果标题；描述默认 **点「一键修复」自动处理**；主钮 **一键修复** |
| `repairing` | 永久隐藏 | 显示 + steps | 主钮 **修复中…**（disabled）或按流程隐藏闪烁 |
| `done_partial` / `done_fail` | 永久隐藏 | 显示 + steps | 主钮仍 **一键修复**（产品无「重试失败项」独立语义则不用） |

合同：

- **禁止** `primaryMap` / 分项修复主钮文案。  
- `#actionBar` 永不显示；状态栏无 attention 主钮。  
- 「稍后处理」：`booting` / `err_bridge` / `idle` 不显示；其余异常可显示（对齐 `canDismiss ≈ bridge_alive && !bootGrace`）。  
- 修复成功全就绪 → 卡消失（不另做成功大卡）。

### 2. 映射卡（与 KeyMappingStage 一致）

- 每张卡 **始终**输出 `.map-card-actions`（录入/手动组合/清除 + 提示句）。  
- 仅 `.map-card.active` 时 CSS 展开（`grid-template-rows: 0fr→1fr`，产品同款）。  
- **移除** `#bindCount` 与 `已绑定 n/m`（产品无此 UI）。  
- hover 只描边。

### 3. 标识

- title / demo-bar：`v5.1` + 注明「对齐产品基线」。  
- 不改 `src/`、`v5.0`。

## [S3] Out of Scope

- 新 UX（半透明、电平折叠、分项修复钮、bindCount）——基线之后再开 feature。  
- 产品 Vue 行为修改。

## Tasks

- [x] T1: 文件与 v5.0 隔离 — 验收: v5.1 存在；v5.0 未改 (covers: S2.3)
- [ ] T2: 修复卡对齐产品 — 验收: 无 `primaryMap`；非 booting 主钮文案为「一键修复」或「修复中…」；action-bar 永 hidden；ready/done_ok 无卡 (covers: S2.1; depends: T1)
- [ ] T3: 映射卡对齐产品 — 验收: actions 始终在模板中；无 bindCount；active 才视觉展开 (covers: S2.2; depends: T1)
- [ ] T4: 更新 verify 脚本 — 验收: 断言无 primaryMap/bindCount；bar 永 false；一键修复存在 (covers: S2; depends: T2, T3)
- [ ] T5: 提交基线 — 验收: commit；脚本 ALL PASS (covers: S2; depends: T4)
