# Voice VibeCoding

基于 [mwlt/Voice_VibeCoding](https://github.com/mwlt/Voice_VibeCoding)（作者 **mwlt**）的 Fork。

---

## 这是什么

Windows 桌面应用：把 **小米遥控器 2 Pro** 接到电脑——

- **按键** → 映射为键盘快捷键  
- **按住语音键** → 遥控器麦克风声音进电脑虚拟声卡，供输入法听写  

---

## 与上游差异（界面以外）

| 项 | 说明 |
| --- | --- |
| **版本策略** | 安装包与代码版本锁 **1.0.x**，不采用上游 1.6.x 版本号。 |
| **按键 / 语音链路** | **保留本 Fork 的 rc003 实现**（统一抑制等）；**不合并**上游 F5 / `voice_dispatch` / hook_bump 等策略。 |

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

上游代码与资源版权属 **mwlt** 及其贡献者。本仓库在其之上的修改归本仓库维护者，遵循上游许可；分发时请保留上游署名。手动组合参考 [LightyearXizIl/Nexus-Prime](https://github.com/LightyearXizIl/Nexus-Prime)，致谢。
