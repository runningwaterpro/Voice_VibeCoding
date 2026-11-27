# WebView2 白屏/黑屏恢复方案

> 最后更新：2026-11-27
> 关联代码：`webview_guard.rs`、`webview_recovery.rs`、`lib.rs`、`ipc/tray.rs`

---

## 症状

- 界面全白或全黑，但**语音、按键映射、桥接功能仍正常**
- 托盘「刷新界面」无效
- 任务管理器中有 `remote-bridge-hub.exe`，但**可能没有**属于本应用的 `msedgewebview2.exe`
- 日志大量 `HRESULT(0x8007139F)`（`ERROR_INVALID_STATE`）

## 根因

Rust 主进程与音频子进程存活，WebView2 **渲染进程已死亡**进入僵尸态。  
`window.reload()` 只能重载 URL，**无法复活已死的 msedgewebview2 进程**。

长期 `window.hide()` 关闭到托盘时，Windows 可能回收隐藏窗口的 WebView2 渲染资源，加速此问题。

## 托盘回收（L0 的强化，v1.2.3 起）

旧 L0 只是 `minimize()` + `set_skip_taskbar(true)`，WebView2 的 6 个进程（约 150MB）
仍常驻，托盘待机一整天等于白白占着内存。

现在改为**主动回收**：最小化静置 30 秒后 `destroy()` 主窗口，点托盘再按需重建。

| 项 | 行为 |
|----|------|
| 触发 | 关闭到托盘、标题栏最小化、启动到托盘（三条路径统一走 `Resized` 事件 + `schedule_ui_recycle`） |
| 延迟 | `UI_RECYCLE_DELAY` = 30 秒；延迟内还原是瞬时恢复 |
| 抑制 | 更新包/驱动包下载中不回收，每 30 秒重试，最多 `UI_RECYCLE_MAX_DEFER` 次 |
| 恢复 | 托盘左键 / 托盘「显示主界面」/ 二次启动 → `restore_main_window` 重建 |
| 显示 | 重建后窗口保持隐藏，前端 `reveal_main_on_frontend_ready` 再显示；3 秒未显示则兜底显示 |
| 一键修复 | 界面已回收时先重建窗口，前端挂好监听后调 `take_pending_tray_repair` 取待办 |
| 与 `hide()` 的区别 | 不是让 Windows 决定何时回收，而是主动销毁后重建，状态可控 |

回收后进程只剩 `remote-bridge-hub.exe`（约 10MB）+ 音频路由子进程（约 1.5MB），
`msedgewebview2.exe` 全部退出；这是**预期状态**，不是白屏故障。

关键日志：

| 关键词 | 含义 |
|--------|------|
| `UI RECYCLE: releasing WebView2 while in tray` | 开始回收 |
| `UI RECYCLE: main window destroyed` | 回收完成 |
| `WINDOW: frontend did not reveal in time` | 前端 3 秒未显示，兜底 show |
| `TRAY: refresh rebuilt the recycled window` | 刷新时窗口本就不在，走了重建 |

---

## 日志位置（Windows）

```
%APPDATA%\com.remote-bridge-hub.app\logs\app.log
```

示例：`C:\Users\<用户名>\AppData\Roaming\com.remote-bridge-hub.app\logs\app.log`

应用内：**设置 → 打开日志目录**（IPC `open_logs_folder`）。

关键日志关键词：

| 关键词 | 含义 |
|--------|------|
| `WEBVIEW GUARD: reloading` | 守卫检测到渲染异常，尝试 reload |
| `failed to reload: WebView2 error` | reload 失败（僵尸态） |
| `WEBVIEW GUARD: recreating` | 升级 destroy + 重建窗口 |
| `WEBVIEW RECOVERY: main window recreated` | recreate 成功 |
| `TRAY: restarting application` | 用户触发「重启软件」 |

---

## 三级恢复 ladder

| 级别 | 手段 | 触发条件 |
|------|------|----------|
| L0 预防 | 关闭 / 启动进托盘 → `minimize()` + `set_skip_taskbar(true)`；静置 30 秒后 `destroy()` 回收（**禁止 `hide()`**） | 用户点关闭且「最小化到托盘」开启；或「启动后最小化到托盘」/`--minimized`；或标题栏最小化 |
| L1 轻量 | `window.reload()` | 心跳超时 / 启动宽限期无 pong |
| L2 重建 | destroy + `WebviewWindowBuilder` 重建 | reload 连续失败 ≥2 次 |
| L3 重启 | `app.restart()` 整进程 relaunch | 托盘「重启软件」；L2 仍失败时用户手动 |

---

## 守卫参数

| 常量 | 值 | 说明 |
|------|-----|------|
| `STALE_AFTER` | 15s | 无心跳视为疑似死亡 |
| `FAIL_THRESHOLD` | 3 | 连续 3 次检查后才干预 |
| `RELOAD_COOLDOWN` | 30s | reload 冷却 |
| `RELOAD_FAIL_THRESHOLD` | 2 | reload 失败 2 次升级 recreate |
| `RECREATE_COOLDOWN` | 60s | recreate 冷却 |
| `FIRST_PONG_GRACE` | 45s | 启动后窗口可见但 JS 未就绪的宽限期 |

前端：`App.vue` 每 5s 调用 `webview_ping`；`onMounted` 后 `getCurrentWindow().show()`（配合 `visible:false` 防白屏闪烁）。

---

## 托盘菜单

1. **刷新界面（白屏自救）** — reload，失败则立即 recreate  
2. **重启软件** — 清理桥接/HID/音频后 relaunch（L3）

---

## 真机验证清单

- [ ] 关闭到任务栏，长时间后再打开：界面正常
- [ ] 模拟黑屏：守卫日志出现 recreate 且界面恢复
- [ ] 托盘「刷新界面」：reload 失败时能 recreate 恢复
- [ ] 托盘「重启软件」：进程重启且功能正常
- [ ] 开机自启：无白屏，页面就绪后窗口出现
- [ ] 黑屏时后端（语音/按键）仍可用，恢复后 UI 与后端状态一致

---

## 改动文件

| 文件 | 职责 |
|------|------|
| `src-tauri/src/webview_guard.rs` | 心跳状态机、Grace、Reload/Recreate 判定 |
| `src-tauri/src/webview_recovery.rs` | reload / recreate / restart 执行 |
| `src-tauri/src/lib.rs` | 守卫线程、关闭 minimize、自启错峰 |
| `src-tauri/src/ipc/tray.rs` | 托盘刷新 + 重启软件 |
| `src/App.vue` | 心跳 + 就绪后 show |
| `src-tauri/tauri.conf.json` | 主窗口 `visible: false` |
