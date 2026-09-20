---
feature: ui-v5.2-src
status: delivered
updated: 2026-09-20
branch: feat/ui-demo-v5.2
commits: 69bc721..3022939
---

# 产品 src 对齐 Demo v5.2

## Report

**What was built** — 产品 `src/` 对齐 demo v5.2 的 8 条合同：修复中悬浮卡常驻（灰钮「修复中…」、步骤实时、无「稍后」）、遮罩 `.55`+`blur(2px)`、电平非就绪 `.is-idle`、设置齿轮、未映射虚线 pill，并删除 `.more-ops`/`showActionBar` 死代码。未加 `.is-not-ready`（遮罩已挡点击）。demo 文件未改。

**Verification** — `npm run build` PASS；`npm test` 23/23 PASS。Review：Spec C1–C8 全 PASS，Correctness PASS（无 critical；boot grace+托盘修复 ≤12s 标题走启动文案，与 demo booting 优先级一致），Codebase consistency PASS（引号风格轻微混用，非阻塞）。

**Journey log**
- v5.1 曾故意对齐程序基线；v5.2 仅 demo，`src/` 相对 `gh/main` 零改动——本次补上。
- 遮罩 `position:absolute; inset:0` 已拦点击，不重复加锁层（ponytail）。
- **C5 回退**：电平整卡 `.is-idle` 误伤增益开关（用户装包实测）；已删 idle，电平卡恢复上一版。
- Pre-existing：`repairFinished`/`repairStepLog` ready 后不清，下次异常可能显示上次成功标题；可另开小任务。
- 托盘 `tray-auto-repair` 可在 12s boot grace 内触发：卡不丢，标题暂为「正在启动…」。

## [S1] Problem

Demo `ui-redesign-demo-v5.2.html` 已定稿 6 处体验合同；产品 `src/` 相对 `gh/main` 零改动，安装包仍是旧行为：修复中藏卡、遮罩偏轻、电平不降权、设置无齿轮、未映射无 pill、修复步骤不实时。

## [S2] Design

对齐合同（以 demo 为准，仅改产品）：

| # | 合同 | 验收 |
|---|---|---|
| C1 | 修复中 **卡常驻**：去掉 `autoRepairing` 藏卡；`autoRepairing` 期间无视 `connecting`/`repairDismissed`，仅 `allReady` 收卡 | 一键修复过程中卡不消失；主钮灰「修复中…」 |
| C2 | 修复中 **步骤实时**：`repairStepLog` 有项即渲染（不再要求 `repairFinished`） | 修复中卡内步骤逐条出现 |
| C3 | 修复中 **不给「稍后」**：`canDismiss && !autoRepairing` | 修复中无次钮 |
| C4 | 遮罩 `rgba(0,0,0,.55)` + `blur(2px)`；卡 `border-radius:10px`、阴影收为 `0 12px 32px rgba(0,0,0,.4)` | 视觉与 demo 一致 |
| C5 | ~~非 `ready/voice` 时电平卡 `.is-idle`~~ **用户否决（2026-09-20）**：整卡变暗且禁用增益；**回退为始终正常亮度、始终可点** | 电平卡无 `is-idle`；自动增益任意状态可切换 |
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

- [x] T1: spec — 验收: 本文件 status 设计完成 (covers: S2)
- [x] T2: C1–C4、C8 in `XiaomiSettings.vue` — 验收: 修复中卡在+步骤实时+无稍后；CSS 数值对齐；无 `more-ops` (covers: S2; depends: T1)
- [x] T5: 回退 C5 电平 idle — 验收: 无 `.vol-meters-card.is-idle`；增益始终可点 (covers: S2 C5; depends: T2)
- [x] T3: C6 `SideNav.vue` + C7 `KeyMappingStage.vue` — 验收: 齿轮在；未映射虚线 pill (covers: S2; depends: T1)
- [x] T4: `npm run build` + `npm test` — 验收: 全绿或 PRE-EXISTING 有记录 (covers: S2; depends: T2, T3)
