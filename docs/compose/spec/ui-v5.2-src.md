---
feature: ui-v5.2-src
status: designed
updated: 2026-09-20
branch: feat/ui-demo-v5.2
commits:
---

# 产品 src 对齐 Demo v5.2

## Report

## [S1] Problem

Demo `ui-redesign-demo-v5.2.html` 已定稿 6 夡体验合同；产品 `src/` 相对 `gh/main` 零改动，安装包仍是旧行为：修复中藏卡、遮罩偏轻、电平不降权、设置无齿轮、未映射无 pill、修复步骤不实时。

## [S2] Design

对齐合同（以 demo 为准，仅改产品）：

| # | 合同 | 验收 |
|---|---|---|
| C1 | 修复中 **卡常驻**：去掉 `autoRepairing` 藏卡；`autoRepairing` 期间无视 `connecting`/`repairDismissed`，仅 `allReady` 收卡 | 一键修复过程中卡不消失；主钮灰「修复中…」 |
| C2 | 修复中 **步骤实时**：`repairStepLog` 有项即渲染（不再要求 `repairFinished`） | 修复中卡内步骤逐条出现 |
| C3 | 修复中 **不给「稍后」**：`canDismiss && !autoRepairing` | 修复中无次钮 |
| C4 | 遮罩 `rgba(0,0,0,.55)` + `blur(2px)`；卡 `border-radius:10px`、阴影收为 `0 12px 32px rgba(0,0,0,.4)` | 视觉与 demo 一致 |
| C5 | 非 `ready/voice` 时电平卡 `.is-idle`（`opacity:.35; pointer-events:none`） | 异常/启动电平变淡 |
| C6 | `SideNav` 设置钮：齿轮 SVG +「设置」（路径抄 demo） | 状态栏右侧有齿轮 |
| C7 | `KeyMappingStage` `.map-bind.unbound`：透明底 + `1px dashed #3a424e` pill | 未映射可见虚线框 |
| C8 | 删 `.more-ops` 死 CSS 及 reduced-motion 引用；删未使用的 `showActionBar` | 源码无 `more-ops` |

实现约束：
- 遮罩层本身 `position:absolute; inset:0` 已挡住下层点击 → **不**另加 `.is-not-ready` 锁（ponytail：重复机制）。
- 不改一键修复逻辑、映射三键 DOM、无 bindCount、无分项修复钮。

## [S3] Out of Scope

- demo v5.0/v5.1/v5.2 文件
- Rust / 后端
- 顶栏异常条复活
- `SettingsSheet` 内部布局

## Tasks

- [ ] T1: spec — 验收: 本文件 status 设计完成 (covers: S2)
- [ ] T2: C1–C5、C8 in `XiaomiSettings.vue` — 验收: 修复中卡在+步骤实时+无稍后；CSS 数值对齐；无 `more-ops` (covers: S2; depends: T1)
- [ ] T3: C6 `SideNav.vue` + C7 `KeyMappingStage.vue` — 验收: 齿轮在；未映射虚线 pill (covers: S2; depends: T1)
- [ ] T4: `npm run build` + `npm test` — 验收: 全绿或 PRE-EXISTING 有记录 (covers: S2; depends: T2, T3)
