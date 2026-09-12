# Voice VibeCoding

**上游项目**：[mwlt/Voice_VibeCoding](https://github.com/mwlt/Voice_VibeCoding)  
**原作者**：**mwlt**（亦见 [Gitee](https://gitee.com/mwlt/remote-voice-vibe-coding)）  
本仓库为其 **Fork**，在此致谢。上游后续版本请看原仓库。

**手动组合快捷键**：交互与实现思路借鉴并行项目 [LightyearXizIl/Nexus-Prime](https://github.com/LightyearXizIl/Nexus-Prime) 中的快捷键组合器（修饰键 + 主键选择、不依赖系统键盘钩子的录入方式）。本仓库已按小米遥控与 Tauri/Vue 结构落地，致谢原实现。

---

## 这是什么

Windows 桌面应用：把 **小米遥控器 2 Pro** 接到电脑——

- **按键** → 映射为键盘快捷键  
- **按住语音键** → 遥控器麦克风声音进电脑虚拟声卡，供输入法听写  

技术栈：**Rust + Tauri 2 + Vue 3**。不是 Python 版 Remote Bridge Hub。

---

## 版本约定

| 项 | 约定 |
| --- | --- |
| 安装包版本 | **1.0.x**（与本仓库 tag 对齐） |
| 不采用 | 上游 **1.6.x** 版本号 |
| 分支 | 主线开发在功能分支（如 `ui/workbench`），稳定后合入 `main` |

---

## 界面（当前方向）

- 深色机箱布局：左「运行状态」+ 右「按键映射」  
- 顶栏：版本、会话状态、电池、设置、退出  
- 双电平（输入 / 送声）+ 增益  
- 分项修复：虚拟声卡 / 虚拟键盘 / ATVV / 重启桥接（异常时高亮主操作）  
- 设计稿：`docs/design/`（v3 / v4.1）

---

## 主要能力

- 小米 2 Pro 蓝牙连接与自动重连  
- 按键映射、录入 / 手动组合快捷键  
- ATVV 语音通道、VB-CABLE 虚拟声卡  
- WinUHid 虚拟键盘（输入法按住听写依赖）  
- 系统托盘（就绪 / 异常 / 初始化）  
- 输入法快捷设置入口  

按键与语音链路 **保留本 Fork 的 rc003 实现**（统一抑制等），不合并上游部分策略重构。

---

## 开发

```bash
npm install
npm run dev      # 前端
npm run build    # vue-tsc + vite
npm test
npm run tauri:build
```

日志：`%APPDATA%\com.remote-bridge-hub.app\logs\app.log`  
修复类操作会写入 `XIAOMI ATVV` / `VOICE env` / `WINUHID` / `restart` 等便于排查的行。

---

## 许可与归属

代码与资源版权归属原作者与各自贡献者。Fork 内修改遵循上游既有许可；发布时请保留上游署名。

相关项目（仅供溯源）：

- [mwlt/Voice_VibeCoding](https://github.com/mwlt/Voice_VibeCoding) · Rust 主上游  
- [LightyearXizIl/Nexus-Prime](https://github.com/LightyearXizIl/Nexus-Prime) · 手动组合快捷键参考
