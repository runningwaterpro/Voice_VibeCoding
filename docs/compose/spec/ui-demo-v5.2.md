---
feature: ui-demo-v5.2
status: in-progress
updated: 2026-09-18
branch: feat/ui-demo-v5.1
commits:
---

# UI Demo v5.2（剃刀优化 · 不改交互模型）

## Report

## [S1] Problem

v5.1 已是对齐产品的基线（居中卡 + 一键修复 + 无顶栏异常条）。仍存在：

1. demo 内 `#actionBar` / more-ops 死代码。  
2. 健康态多处重复「正常」文案。  
3. 非就绪态满幅死电平占首屏。  
4. 单按钮异常遮罩过重。  
5. 圆角/阴影/映射提示句偏噪，美观未收口。

必须在 **`ui-redesign-demo-v5.2.html`** 上改，**不改** v5.0 / v5.1。

## [S2] Design

工作区：`.worktrees/ui-demo-v51` · 自 `ui-redesign-demo-v5.1.html` 复制。

### P0 删与去重

- 删除 `#actionBar` 及 more-ops、`#btnPrimary`、对应 CSS/JS；`applyState` 不再触碰 action-bar。  
- 健康态（`ready`/`voice`/`done_ok`）：状态栏仅 LED + headline，强制隐藏 `#subline`。  
- 异常/启动：允许 sub；居中卡仍是唯一操作面（一键修复不变）。

### P1 电平与遮罩

- `ready`/`voice`：电平常显。  
- 其它态：`.meters` 加 `.is-idle`（`opacity:.35` + 非交互感）。  
- **遮罩加重（用户定稿）**：未就绪卡出现时 `background: rgba(0,0,0,.55)` + 轻 `blur(2px)`，凸显悬浮卡、压暗主界面；**不要**更透明。  
- `.work` 继续 `is-not-ready` 锁死（`pointer-events:none`）。

### P1b 未就绪锁（用户确认的统一原则）

原则：**主界面「看起来可用」⇔ 真正就绪**；未就绪 = 卡在 + 主内容锁死可见。

| 状态 | 卡 | 主内容 `.work` | 主钮 |
|---|---|---|---|
| ready / voice / done_ok | 无 | 正常可点 | — |
| booting | 有 · 无钮 | 锁 | 无 |
| 异常 | 有 | 锁 | 一键修复 |
| **repairing** | **有（不消失）** | **锁** | **`修复中…` disabled（DOM 常驻，不闪）** |
| done_partial / done_fail | 有 · 步骤 | 锁 | 一键修复 |

实现：`showModal = !healthy`（含 repairing）；`.app.is-not-ready .work{pointer-events:none}`；repairing 分支 title/desc/steps + `btn.disabled=true`、文案「修复中…」、隐藏「稍后」。

### P2 美观

- 圆角 10px 单层阴影；映射提示单句；`.bind.unbound` 虚线 pill。  
- **设置按钮：齿轮 SVG +「设置」**（用户选定；非纯图标）。  
- 不改：一键修复文案结构、映射 actions 常驻 DOM、无 bindCount、无分项修复钮。

### 标识

- title / demo-bar：`v5.2`。

## [S3] Out of Scope

- 改 v5.0 / v5.1；产品 `src/`（可另开）；分项修复钮；bindCount。

## Tasks

- [x] T1: 复制为 v5.2 并改标识 — 验收: 新文件存在；v5.1 不动 (covers: S2)
- [x] T2: P0 — 验收: 无 `id="actionBar"`；ready 无 subline (covers: S2 P0; depends: T1)
- [x] T3: P1 — 验收: 非 ready/voice 电平 `.is-idle`；遮罩无 blur (covers: S2 P1; depends: T1)
- [x] T4: P2 + 脚本 — 验收: verify ALL PASS (covers: S2 P2; depends: T2, T3)
- [ ] T5: **P1b** — 验收: repairing 卡可见+主钮 disabled「修复中…」；`.work` 锁；ready 无卡可点映射 (covers: S2 P1b; depends: T4)
- [ ] T6: 提交 P1b — 验收: commit；verify 更新 ALL PASS (covers: S2; depends: T5)
