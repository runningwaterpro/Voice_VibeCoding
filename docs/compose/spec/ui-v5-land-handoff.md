# ui-v5-land 交接说明（可续跑）

> 供下一个规划/会话直接续做。本文件只记录事实与下一步，不含探索史。

## 目标（用户已定）

1. 在 `feat/ui-v5-land` 上触发远程 GitHub Actions，产出 NSIS 安装包 artifact。  
2. **用户自己**看 CI 是否完成、自己下载安装验证。  
3. 验证通过后，用户再说「合并 main」，再合。  
4. **不要**轮询 CI、不要自动合并、不要在本轮 Finish 里 merge。

## 已完成

| 项 | 状态 |
|---|---|
| Spec | `docs/compose/spec/ui-v5-land.md` · `status: delivered` |
| 分支 | `feat/ui-v5-land` |
| 工作区 | `D:\dev\rust\Voice_VibeCoding-sync\.worktrees\ui-v5-land` |
| base | `cdd0a33` |
| head（功能+spec） | `e4d7cd8`（此前）；若本轮 dispatch 后无新提交则仍为此 SHA，以 `git log -1` 为准 |
| 远端 | 已推 `gh/feat/ui-v5-land`（GitHub `runningwaterpro/Voice_VibeCoding`） |
| 本地验证 | `npm run build` PASS；`npm test` PASS（4 files / 23 tests） |
| 实现范围 | 无边框 `decorations:false`（conf+recreate）；`WindowTitlebar`；状态栏去退出；高度上限 900；映射列灰线/composer 连线规则 |

## CI 为何此前没跑（根因）

`.github/workflows/build.yml` 触发条件：

```yaml
on:
  push:
    branches: [main, "fix/*"]
    tags: ["v*"]
  release:
    types: [published]
  workflow_dispatch:
```

- push 到 **`feat/*` 不匹配** `main` / `fix/*` / `v*` → **push 不会起 run**。  
- 有 `workflow_dispatch`，可对任意 ref 手动触发。  
- Artifact 名：`Voice-VibeCoding-NSIS`（`upload-artifact`，非 Release 资产）。  
- Release 上传仅在 tag `v*` 或 release published 时执行；分支 CI **只出 artifact，不发 Release**。

## 下一步（按顺序，不要发散）

1. **确认 CI 已触发**（用户自己看，或下一任会话只查一次）：  
   - GitHub → Actions → `build-nsis` → 应出现 `feat/ui-v5-land` 的 run。  
   - 或：`gh run list --repo runningwaterpro/Voice_VibeCoding --branch feat/ui-v5-land`（本机可能无 `gh`，用网页即可）。  
2. 若 **没有** run：用 workflow_dispatch 指 `ref=feat/ui-v5-land`，或把分支改名为 `fix/ui-v5-land` 再 push（匹配现有 `fix/*` 触发）。**不要改 build.yml 扩大分支 unless 用户要求。**  
3. Run 成功后：Artifacts → `Voice-VibeCoding-NSIS` → 下载 exe → 安装验证（标题栏/无退出/托盘退出/映射无灰线）。  
4. 用户回「合并 main」→ 再从**主检出**合并（worktree 持有该分支时勿在别处 checkout main 合并冲突流程见 compose-next Finish）。  
5. 合并后可选：`git worktree remove .worktrees/ui-v5-land`（仅 `.worktrees/` 下）。

## 明确不做

- 不轮询 Actions。  
- 不自动 merge / 不开 PR（除非用户改口）。  
- 不碰主检出 `fix/composer-hide-link` 上的脏文件（README、v4.3、删 spec 等）。  
- 非必要不用子代理。

## 关键路径

- Spec：`docs/compose/spec/ui-v5-land.md`  
- CI：`.github/workflows/build.yml`  
- 标题栏：`src/components/WindowTitlebar.vue`  
- 侧列/连线：`src/components/KeyMappingStage.vue`  
- 窗口 conf：`src-tauri/tauri.conf.json` · recreate：`src-tauri/src/webview_recovery.rs`
