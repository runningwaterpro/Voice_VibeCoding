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
- 其它态：`.meters` 加 `.is-idle`（`opacity:.35` + 非交互感），文案 hint→「待命」/保持 state 数据但降权。  
- `.repair-modal` 背景：`rgba(0,0,0,.35)`，`backdrop-filter` 去掉或 `blur(0)`。

### P2 美观

- 机箱/卡圆角统一 **10px**；修复卡 12px 可保留但阴影只留一层。  
- 映射提示句：`点录入后按目标键或组合键`（单句）。  
- `.bind.unbound`：虚线边框 pill（弱化但可扫）。  
- 不改：一键修复文案、居中卡结构、映射 actions 常驻 DOM、无 bindCount。

### 标识

- title / demo-bar：`v5.2` +「剃刀优化」。

## [S3] Out of Scope

- 改 v5.0 / v5.1；改产品 `src/`；分项修复钮；bindCount。

## Tasks

- [ ] T1: 复制为 v5.2 并改标识 — 验收: 新文件存在；v5.1 哈希不变 (covers: S2)
- [ ] T2: P0 — 验收: 源中无 `id="actionBar"`；ready 无 subline (covers: S2 P0; depends: T1)
- [ ] T3: P1 — 验收: 非 ready/voice 电平 `.is-idle`；遮罩无重 blur (covers: S2 P1; depends: T1)
- [ ] T4: P2 + 脚本 — 验收: verify v5.2 ALL PASS；一键修复仍存在 (covers: S2; depends: T2, T3)
- [ ] T5: 提交 — 验收: commit 含 v5.2 (covers: S2; depends: T4)
