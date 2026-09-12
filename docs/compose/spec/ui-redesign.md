---
feature: ui-redesign
status: in-progress
updated: 2026-09-12
branch: ui/workbench
commits: 
---

# UI Redesign (Xiaomi-first workspace)

## Report

## [S1] Problem

1. 小米页信息过载：状态、波形、修复、键位、增益平铺，全屏也放不下遥控器。
2. 电平标尺与说明占垂直空间；双电平曾挤占布局。
3. 左侧曾出现 T1/V60（开发中）；产品现只支持小米 2 Pro。
4. 用户常「合盖带走再回来」：需要**左侧一眼看到**连接/语音是否就绪，并一键重连/修复。
5. Demo 若脱离真实 IPC，落地会返工。

## [S2] Design

### 信息架构（对齐现有 IPC）

| UI | 真实能力 | 禁止 |
|---|---|---|
| 启动自动连接 | `start_bridge` + `xiaomi_reconnect_loop` | 虚构「手动配对向导」 |
| 断开 / 重新连接 | `stop_bridge` / `start_bridge` | 断开后仍显示「已连接」 |
| 虚拟声卡修复 | `repair_xiaomi_voice_env` / `check_xiaomi_voice_env` | 一键「修好所有」却不调对应命令 |
| 修复虚拟键盘 | `repair_xiaomi_winuhid` | 与声卡/ATVV 混成一个按钮语义 |
| 修复 ATVV | `repair_xiaomi_atvv` | 仅断言「通道好」而不调修复 |
| 重启桥接 | `restart_xiaomi_bridge` | — |
| 键位录入 | `capture_shortcut_start/poll/stop` + 就地展开卡片 | 底部独立录入条（与现交互不一致） |
| 手动设键 | 写入与录入相同的 `KeyAction`（Single/ComboKey → update_key_mapping）；参考并行分支「录入不便时点选组合」 | 独立第二套存储或只改 UI 不落盘 |
| 语音 IME 一键预设 | **本 fork 不做**（用户已从根源解决，拒绝上游需输入法配合的将错就错） | 在 demo/Vue 恢复 VOICE_QUICK_PRESETS |
| 双电平 | `ble_level` 增益前 / `cable_level` 增益后 | 声称采系统声卡 |
| 设置 | `/settings`：自启、托盘、hide_dev_menus | 主界面塞驱动细节 |

**修复不是全量一键**：现有为三条独立修复 + 重启桥接。UI 须保持分项。

**连线**：仅改 `viewBox`，禁止 `setAttribute("width"/"height")`（现网 WebView 抖动）。

**手动设键**：卡片内「手动设置」点选修饰键+主键生成组合，与「录入」并列；应用写入与录入相同的 mapping。

**IME 语音预设**：明确 out of scope（本 fork 已根源解决语音唤醒，不需要用户再配输入法快捷键）。

### 动效合同（落地验收项）

| 元素 | 动效 | 时长 / 曲线 |
|---|---|---|
| 可点按钮 | `:active` → `scale(0.97)`；仅 `transform`/`background`/`border`/`color` | 160ms / `cubic-bezier(0.23,1,0.32,1)` |
| 键帽 hover | 背景/描边/字色 | 150ms / 同上 · 仅 `hover:hover` |
| 映射卡展开 | `grid-template-rows: 0fr → 1fr`（内层 overflow hidden） | 180ms ease-out |
| 手动组合面板 | opacity + `translateY(6px→0)` | 进 200ms / 出 120ms |
| 连线 | 仅 `opacity` | 120ms · **禁止** SVG width/height、path morph |
| 电平游标 | `left` linear | 70ms |
| 开关 | `transform` + 背景 | 150ms |
| 高级诊断展开 | `grid 0fr/1fr` | 200ms |
| `prefers-reduced-motion` | 去掉位移与缩放，保留颜色/opacity | — |

**禁止**：`transition: all`；键盘触发长动画；电平 spring；从 `scale(0)` 出现。

Demo `ui-redesign-demo-v3.html` 已含上述合同，作动效对照源。

### 自适应

- &lt;860：快捷栏在上，遥控器与映射纵向。
- ≥860：快捷 | 左映射 | 遥控器 | 右映射。
- ≥1180：映射双列仍保持左右对遥控器。
- 电平轨 `minmax` 拉长；ResizeObserver 重算连线。

### 错误类型 → 高亮修复按钮（对齐 `xiaomi_host_status_now`）

后端已有优先级链（`commands.rs`）。前端**只读** `XiaomiHostStatus` 字段，按同一优先级高亮**一个**主按钮，可附加次要建议：

| 条件（与 Rust 同序） | status | 主高亮按钮 | 次要 |
|---|---|---|---|
| `bridge && audio && cable && winuhid && atvv` | 运行正常 | 无 | — |
| `bridge && !winuhid` | 虚拟键盘未就绪 | **修复虚拟键盘** | — |
| `bridge && !atvv`（且键盘已 ok） | ATVV 未连接 | **修复 ATVV 连接** | — |
| `!cable` | 语音环境未就绪 | **虚拟声卡修复** | — |
| `bridge && !audio` | 语音路由未就绪 | **重启桥接** | 虚拟声卡修复 |
| `!bridge` | 桥接未运行 | **重新连接 / 重启桥接** | — |
| 其它组合 | 部分服务异常 | **重启桥接** | 虚拟声卡修复 |

实现：`attention: RepairId[]`，最多 1 个 `primary` + 可选 `secondary`；托盘仍用现有三态（Ready/Error/Init）。禁止把多条修复合并成一个「一键全修」按钮。

### V4 异常态 UI 合同（左栏 + 顶栏）

对照 `ui-redesign-demo-v4.html` 的 `STATES` / `applyState`：

**左栏结构（自上而下）**
1. 连接按钮 `btnConn`：`断开遥控器`（ghost）/ `重新连接`（primary）/ `取消连接`（ghost）
2. `status-line`：粗体 headline + 灰色 sub
3. 四枚 chip：蓝牙 / 语音通道 / 虚拟声卡 / 虚拟键盘（ok 默认绿、`warn`/`fail`/`idle`）
4. `repair-group`：仅显示 **primary**（`.btn.attention`）与 **secondary**（`.btn.attention-sec`）；其余修复不出现
5. `healthy-note`：健康时「全部服务正常」；采音中「采音送声中」；异常时隐藏
6. `more-ops`（details）：始终可展开，内含全部 4 个强制修复 + 输入法设置
7. `credit-line`：上游 mwlt 署名

**顶栏**：品牌 · LED+一句话状态 · 设置 · 退出（连接钮只在左栏，不在顶栏）

**状态 → 按钮映射**

| 状态 | LED | headline | sub | primary | secondary | 备注 |
|---|---|---|---|---|---|---|
| idle | dim | 未连接遥控器 | 点「重新连接」或等待自动重连 | conn=重新连接(primary) | restart | chips 全 idle |
| connecting | warn | 正在连接遥控器… | 正在建立蓝牙与语音通道 | 无 | 无 | conn=取消连接(ghost)；隐藏修复 |
| ready | ok | 语音可用 | — | 无 | 无 | healthy-note；chips 全 ok |
| voice | ok | 采音中 | 松开语音键结束 | 无 | 无 | healthy-note=采音送声中 |
| error(ATVV) | warn | 语音通道未就绪 | 点「修复 ATVV 连接」 | atvv | — | atvv chip fail |
| err_hid | warn | 虚拟键盘未就绪 | 语音唤醒需要虚拟键盘 | winuhid | — | hid fail；atvv warn |
| err_cable | warn | 虚拟声卡未就绪 | 点「虚拟声卡修复」 | cable | — | cable fail |
| err_route | warn | 语音路由未就绪 | 点「重启桥接」或「虚拟声卡修复」 | restart | cable | |
| err_bridge | fail | 桥接未运行 | 点「重新连接」或「重启桥接」 | conn=重新连接(primary) | restart | |

**高级诊断四块**：主机状态 · 修复动作（4 个始终可点）· 实时日志 · 原始状态字段（JSON）。

### 布局（对齐 KeyMappingStage + 左侧快捷）

```
┌ 顶栏：品牌 · 一句话状态 · 设置 · 退出 ────────────────┐
│ [输入/送声 双电平，各一行、矮轨]                      │
├ 运行状态(左) ┬ 左映射卡 ┬ 遥控器 ┬ 右映射卡 ─────────┤
│ 与列同高      │ 卡片均分  │ 垂直居中│ 均分            │
│ 连接/修复/增益│ 就地录入  │        │                 │
├ 高级诊断（可折叠）：主机状态 · 修复动作 · 日志 · 原始字段 ┤
└──────────────────────────────────────────────────────┘
```

- **无设备栏**；仅小米。
- **双电平两轨等宽**：输入/送声共用同一 `grid-template-columns`；**增益独立第三行**，不得挤占送声轨导致两轨不可比。
- **修复按钮渐进披露**：健康时**不显示**修复按钮，仅「全部服务正常」+ 可折叠「更多操作」；异常时只显示**当前需要**的修复按钮并高亮；全部修复动作始终可在「更多操作」/高级诊断中找到。
- **运行状态栏在左**、与 stage 列同高。
- **按键映射**为**单一外框**（`map-work`）：标题 + 左键位列 | 遥控器 | 右键位列；框内有内边距，避免键位卡与外框贴死、列间散乱。
- **映射列**在框内 `space-evenly` 均分；遥控器垂直居中。
- **连线**：按键边中点 → 卡片边中点；`pointer-events:none`。
- **文案**：产品区只用用户语言；禁止设计过程用语与 IPC 函数名。

### 自适应

- &lt;860：快捷栏在上，遥控器与映射纵向。
- ≥860：快捷 | 左映射 | 遥控器 | 右映射。
- ≥1180：映射双列仍保持左右对遥控器。
- 电平轨 `minmax` 拉长；ResizeObserver 重算连线。

### 视觉

石墨机箱 + 信号青绿/琥珀/砖红；遥控器沿用 `RemoteHotspot` 浅色机身 + 深键（实物感），避免整页黑底塑料感。

## [S3] Out of Scope

- rc003 按键/抑制逻辑
- 版本号策略、静默升级、startup_env 流水线
- T1 / V60 页面（不展示）
- 在未确认前改正式 Vue 组件

## Tasks

- [x] T1: 写出技术对齐 demo v3（完整 13 键、左快捷栏、就地录入、贴边连线、分项修复）— acceptance: 打开 HTML 可演示；文案与 IPC 一致 (covers: S2)
- [x] T2: 用户确认设计后，再拆 Vue 落地任务 — acceptance: 另开 implement 任务 (covers: S2; depends: T1)
- [x] T3: V4 正常态布局（顶栏/双电平/左栏/键位定宽/设置 sheet）— acceptance: build+截图对照 V4 ready (covers: S2)
- [x] T4: V4 异常态状态机（idle/connecting/ready/voice/ATVV/hid/cable/route/bridge）+ 主/次高亮 + 四芯片 + 更多操作 + 高级诊断四块 — acceptance: `scripts/verify_v4_states.py` 八态 attention/chips/healthy/conn 与 V4 一致 (covers: S2 错误类型表; V4 异常态 UI 合同)
- [x] T5: 后续修正：dark hover、attention-sec 琥珀、桥接双主操作、电池 chip、电平白线、设置开关无叉、composer 2 列、adv 等高、去掉健康重复文案、stop/start 等待 running — acceptance: verify_v4_states 八态与更新后 V4 一致 (covers: S2; 用户实测反馈)
