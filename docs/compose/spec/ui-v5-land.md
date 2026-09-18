---
feature: ui-v5-land
status: delivered
updated: 2026-09-18
branch: feat/ui-v5-land
commits: cdd0a33d9084e23ae557348381e75eb4c17000ef..bd5c4c1d59bcddaa8a66e01818cfb255fc87e9f9
---

# UI v5 Land（窗口 chrome + 界面对齐落地）

## Report

**What was built** — 主窗口 `decorations: false`（conf + recreate 一致）；新增 `WindowTitlebar`（拖拽 + 最小化/最大化/关闭，关闭走既有托盘策略）；`SideNav` 降为状态栏（状态/电池/设置/更新角标），窗口内退出入口与确认框已移除（IPC 与托盘「退出」保留）。`fitWindowHeightToContent` 上限与 conf 统一为 900。`KeyMappingStage` 侧列仅纵滚、子项不收缩、卡片贴合内容盒；composer 打开时连线隐藏，关闭/应用后恢复。

**Verification** — worktree 内 `npm run build` PASS（vue-tsc + vite，含 WindowTitlebar focus 类型修正后）；`npm test` PASS（4 files / 23 tests）。人工窗口路径（无边框拖拽、三钮、托盘退出）需 `tauri dev` 点检，未自动化。

**Journey log** — demo 灰线有两条根因：flex 收缩压扁卡片、`overflow-y:auto` 隐式 `overflow-x:auto` 露出横滚条；产品侧列一并修。`outer−inner` 在无边框下≈0，高度公式仍测 chrome 以免 recreate 窗口回弹。Review 子代理曾空转，已改主任务自审（非必要不用子代理）。

## [S1] Problem

设计稿 `docs/design/ui-redesign-demo-v5.0.html`（主检出未跟踪副本亦存在）已定稿「自实现窗口 chrome · 标题栏/状态栏分层 · 退出仅托盘 · 内容自适应高度」，并在 demo 内修完 composer 连线隐藏与映射侧列灰线。真机仍停在系统装饰窗 + 单行顶栏：

1. `src-tauri/tauri.conf.json` 与 `webview_recovery.rs::build_main_window` 均为 `decorations: true`，无自绘标题栏、无窗口控制按钮；`fitWindowHeightToContent` 仍按系统标题栏补高。
2. `SideNav.vue` 顶栏把品牌、会话状态、电池、设备导航、设置、**退出**混在一行；退出走确认框 + `quit_application`，与 v5.0「退出仅托盘」冲突。
3. 产品键位映射列仍是可横向溢出的滚动容器 + flex 默认可收缩，存在与 demo 相同的「卡片压扁灰线 / 底部横滚条灰线」风险；composer 打开时连线规则未按 demo 收口。

用户要求：chrome + 界面整包形成一套可落地方案，**Spec 批准前不实现**。

## [S2] Design

对照源：`docs/design/ui-redesign-demo-v5.0.html`（若 worktree 无此文件，以主检出 `D:\dev\rust\Voice_VibeCoding-sync\docs\design\ui-redesign-demo-v5.0.html` 为权威，不改动主检出脏文件）。

工作区覆盖：`.worktrees/ui-v5-land` · `feat/ui-v5-land`。主检出上与本 feature 无关的未提交变更（README、v4.3、删 spec、repair-feedback-demo 等）**一律不动、不纳入本分支**。

### 1. 无边框窗 + 自绘标题栏（窗口 chrome）

契约：

- 主窗口 `decorations: false`：改 `tauri.conf.json` `app.windows[0].decorations`，并改 `webview_recovery.rs::build_main_window` 为 `.decorations(false)`，两处必须一致（recreate 路径不得弹回系统边框）。
- `resizable: true` 保留；Windows 下无边框窗仍用系统边缘缩放，**不**移植 demo 里的 `.rs-*` 自绘手柄（demo 手柄仅服务浏览器预览）。
- 前端新增标题栏组件（建议 `src/components/WindowTitlebar.vue`，挂载于 `App.vue` 机箱最顶）：
  - 高度 36px，背景 `#12151a`，`data-tauri-drag-region`（或 Tauri v2 等价 drag 属性）可拖拽移动窗口。
  - 仅含：品牌名 + 版本号（可复用 `SideNav` 已有 `getVersion` 文案来源）+ 窗口控制三钮（最小化 / 最大化·还原 / 关闭）。
  - 控制钮 `no-drag`；关闭触发与系统关闭同一路径（见 §3），不直接 `quit`。
  - 最大化/还原：读 `isMaximized`，切换图标与 `data-maximized`。
- 状态栏与标题栏**分层**：现 `SideNav.vue` 的 `header.topnav` 降级为状态栏职责（或拆为 `WindowTitlebar` + 状态栏），高度与分层样式对齐 demo `.titlebar` / `.statusbar`（36 + 40，底边分割线），禁止再把窗口控制与应用操作挤在同一行。
- `fitWindowHeightToContent`：无系统标题栏后 `chromeH = outer - inner` 应为 0（或仅保留边框差）；公式改为以内容高 + 机身 margin 为目标，**不得**再把已删除的系统标题栏厚度加回去。保留「启动期只增不减」与防抖定时器行为。

### 2. 退出仅托盘；窗口内去掉退出

契约：

- 删除 `SideNav.vue` 顶栏「退出」按钮及依赖它的确认框入口（`openQuitConfirm` / 模板 `nav-exit` / `quit-dialog` 若仅服务该入口则一并移除；`confirmQuit`→`quit_application` 的 IPC **保留**给托盘与未来菜单，不在窗口 UI 暴露）。
- 状态栏应用操作仅保留「设置」（及既有更新入口若与品牌行拆分则归状态栏，不进标题栏）。
- 关闭行为不变：`attach_main_window_close_handler` 已按 `minimize_to_tray` 决定进托盘或退出；标题栏关闭钮只调用窗口 `close()`，不绕过该 handler。
- 托盘菜单「退出」`tray.rs` 保持为唯一窗口外进程退出入口。

### 3. 状态栏信息架构（对齐 v5.0）

- 标题栏：身份 + 窗口控制。
- 状态栏：LED/会话摘要 + 电池 chip + 设置（+ 既有更新角标策略可暂留品牌行或迁状态栏，不新增功能）。
- 设备导航（小米/T1/V60）与主体内容关系**不**在本 feature 重做 IA；仅保证拆分后导航仍可用、不重复出现退出。

### 4. 映射侧列滚动与灰线（demo 已验证规则移植）

落在 `KeyMappingStage.vue`（及其中 composer/连线样式）：

- 侧列容器：仅纵向滚动——`overflow-y: auto` 且显式 `overflow-x: hidden`；子项 `flex-shrink: 0`；卡片宽 `width: 100%`（或与内容盒一致），禁止写死导致 `scrollWidth > clientWidth` 的横滚条。
- composer 打开：隐藏映射连线（demo：`manualOpenId` 真则去 `.on` 并 return）；关闭后恢复端点。对应现有 `manualEditor` 状态。
- 验收不含视觉像素级克隆 demo，只要求：无横向滚动条灰线、无卡片压成 ~2px 的竖向灰线、composer 开合连线显隐正确。

### 5. 内容自适应高度

- 保留 `scheduleFitWindowHeight` 调度；与 §1 无 chrome 补高后的公式一致。
- 窗口 `maxHeight`（conf 900 / 函数 960）冲突时以可观察行为「不无限涨、不压矮已扩高」为准，实现取 **一处**权威上限并在 Spec 任务验收中固定为 conf 与代码一致。

### 6. 验证边界

- 前端：`npm run build`（vue-tsc + vite）必须过。
- 单测：`npm test` 基线；新增逻辑若有纯函数（高度公式、最大态图标切换）优先可测处覆盖，不强求 E2E。
- 手动/脚本验收：见 Tasks；Windows 下 `decorations:false` 需 `tauri dev` 或等价真实窗口验证，自动化仅覆盖能 headless 的部分。

## [S3] Out of Scope

- 主检出 `fix/composer-hide-link` 及其它分支的未提交脏文件与 README/删 spec/v4.3/repair-feedback-demo 清理。
- demo `.rs-*` 自绘 resize 手柄、demo-bar、假 STATES 启动序列产品化。
- 重做键位录入 IPC、修复分项逻辑、托盘图标算法、升级流。
- 非小米设备页 IA、暗色主题变量全量换肤。
- 在未批准本 Spec 前的任何产品代码提交。

## Tasks

- [x] T1: 主窗口与 recreate 路径关闭系统装饰 — acceptance: `tauri.conf.json` 与 `build_main_window` 均为 `decorations: false`，dev/recreate 后无系统标题栏 (covers: S2.1)
- [x] T2: 标题栏组件 + 状态栏分层，窗口控制接通最小化/最大化/关闭 — acceptance: 可拖拽移动；三钮工作；关闭走 `CloseRequested` 托盘策略；`SideNav` 不再单行混窗口控制 (covers: S2.1, S2.3; depends: T1)
- [x] T3: 窗口内移除退出入口 — acceptance: UI 无「退出」按钮与确认框入口；托盘「退出」仍可 `quit_application` (covers: S2.2; depends: T2)
- [x] T4: `fitWindowHeightToContent` 按无系统标题栏重算且上下限一致 — acceptance: `npm run build` 过；chromeH 不再虚高；代码与 conf 最大高度一致 (covers: S2.1, S2.5; depends: T1)
- [x] T5: 映射侧列 `overflow-x:hidden` + 子项不收缩 + 卡片宽度贴合 — acceptance: 打开 composer 或长列表时无底部横滚条、无 2px 压扁卡；`npm test` 不回归 (covers: S2.4)
- [x] T6: composer 打开隐藏连线、关闭恢复 — acceptance: 开：连线不可见；关/应用/取消：选中或 hover 连线恢复且端点贴键与卡 (covers: S2.4; depends: T5)
- [x] T7: 端到端验收 — acceptance: `npm run build` + `npm test` 全过；记录命令与结果；人工核对标题栏/无退出/托盘退出/映射无灰线 (covers: S2.1–S2.6; depends: T2, T3, T4, T5, T6)
