<h1 align="center">
  <img src="https://raw.githubusercontent.com/Houyusu/claude-halo/master/docs/halo-icon.svg" width="64" height="64" alt=""/><br>
  Claude Halo
</h1>

<p align="center">
  <strong>桌面光环指示器</strong> — 屏幕右下角的彩色光环，实时反映 Claude Code 运行状态。
  <br>余光即可感知，无需切换窗口。
</p>

<p align="center">
  <a href="https://houyusu.github.io/claude-halo/"><strong>🎬 在线演示 →</strong></a>
  &nbsp;·&nbsp;
  <a href="https://github.com/Houyusu/claude-halo"><strong>GitHub →</strong></a>
</p>

<p align="center">
  <img src="https://img.shields.io/badge/platform-Windows-0078D6?style=flat-square&logo=windows" alt="Windows">
  <img src="https://img.shields.io/badge/plugin-Claude%20Code-ff8830?style=flat-square" alt="Claude Code Plugin">
  <img src="https://img.shields.io/badge/license-MIT-33cc55?style=flat-square" alt="MIT">
  <img src="https://img.shields.io/badge/version-1.0.7-9944ff?style=flat-square" alt="v1.0.7">
</p>

---

## 📡 六种状态

光环颜色和动画随 Claude Code 状态自动变化，每个状态有独特的视觉节奏：

<table>
<tr>
  <td align="center" width="110">
    <svg width="48" height="48" viewBox="0 0 48 48"><circle cx="24" cy="24" r="18" fill="none" stroke="#aaaaaa" stroke-width="3" stroke-dasharray="30 22" stroke-linecap="round"><animateTransform attributeName="transform" type="rotate" from="0 24 24" to="360 24 24" dur="6s" repeatCount="indefinite"/></circle></svg>
    <br><code style="color:#aaaaaa">#aaaaaa</code>
    <br><b>待命</b>
    <br><sub>等待你的输入</sub>
  </td>
  <td align="center" width="110">
    <svg width="48" height="48" viewBox="0 0 48 48"><circle cx="24" cy="24" r="18" fill="none" stroke="#ff8830" stroke-width="3" stroke-dasharray="35 18 22 15 12 10" stroke-linecap="round"><animateTransform attributeName="transform" type="rotate" from="0 24 24" to="360 24 24" dur="2.4s" repeatCount="indefinite"/></circle></svg>
    <br><code style="color:#ff8830">#ff8830</code>
    <br><b>思考</b>
    <br><sub>Claude 正在推理</sub>
  </td>
  <td align="center" width="110">
    <svg width="48" height="48" viewBox="0 0 48 48"><circle cx="24" cy="24" r="18" fill="none" stroke="#3399ff" stroke-width="3" stroke-dasharray="25 13 10 10 17 13 13 11" stroke-linecap="round"><animateTransform attributeName="transform" type="rotate" from="0 24 24" to="360 24 24" dur="1.3s" repeatCount="indefinite"/></circle></svg>
    <br><code style="color:#3399ff">#3399ff</code>
    <br><b>执行</b>
    <br><sub>正在调用工具</sub>
  </td>
</tr>
<tr>
  <td align="center" width="110">
    <svg width="48" height="48" viewBox="0 0 48 48"><circle cx="24" cy="24" r="18" fill="none" stroke="#ee3333" stroke-width="3" stroke-dasharray="40 25 15 13" stroke-linecap="round"><animateTransform attributeName="transform" type="rotate" from="0 24 24" to="360 24 24" dur="2.8s" repeatCount="indefinite"/></circle></svg>
    <br><code style="color:#ee3333">#ee3333</code>
    <br><b>等待输入</b>
    <br><sub>需要你的响应</sub>
  </td>
  <td align="center" width="110">
    <svg width="48" height="48" viewBox="0 0 48 48"><circle cx="24" cy="24" r="18" fill="none" stroke="#33cc55" stroke-width="3" stroke-dasharray="35 18 22 15 12 10" stroke-linecap="round"><animateTransform attributeName="transform" type="rotate" from="0 24 24" to="360 24 24" dur="5s" repeatCount="indefinite"/></circle></svg>
    <br><code style="color:#33cc55">#33cc55</code>
    <br><b>完成</b>
    <br><sub>任务执行完毕</sub>
  </td>
  <td align="center" width="110">
    <svg width="48" height="48" viewBox="0 0 48 48"><circle cx="24" cy="24" r="18" fill="none" stroke="#9944ff" stroke-width="3" stroke-dasharray="17 10 17 10 17 10" stroke-linecap="round"><animateTransform attributeName="transform" type="rotate" from="0 24 24" to="360 24 24" dur="2.1s" repeatCount="indefinite"/></circle></svg>
    <br><code style="color:#9944ff">#9944ff</code>
    <br><b>压缩</b>
    <br><sub>正在整理上下文</sub>
  </td>
</tr>
</table>

<p align="center">
  <em>点击 <a href="https://houyusu.github.io/claude-halo/">在线演示</a> 体验完整动画效果（含颜色切换与光环呼吸）</em>
</p>

---

## 🔧 安装

在 Claude Code 中输入：

```bash
/plugin marketplace add Houyusu/claude-halo
/plugin install claude-halo@claude-halo
```

安装后重新启动 Claude Code，光环自动出现在屏幕右下角。

### 卸载

```bash
/plugin uninstall claude-halo
```

---

## 🖱️ 操作

| 操作 | 效果 |
|------|------|
| **右键光环** | 打开菜单 — 切换穿透 / 退出 |
| **左键光环** | 显示当前状态摘要 |
| **Ctrl+Shift+F12** | 快速切换点击穿透 |
| **拖拽光环** | 移动光环位置 |

---

## 🏗️ 架构

```
SessionStart → launch-halo.ps1 → claude-halo.exe (Tauri v2)
                                      │
Hook events ──→ halo-hook.ps1 ──→ TEMP\claude-halo-state.txt
                                      │
                              ┌───────┘ (150ms poll)
                              ▼
                    Rust 状态机 + Canvas 渲染
                              │
              Toolhelp32 ──→ 检测 claude.exe 存活
              (不依赖 hook 执行，关终端自动退出)
```

- **后端**: Rust + Tauri v2，Windows 原生窗口（透明、置顶、无任务栏）
- **渲染**: Canvas 2D，三圈层光环（外辉光 + 中间层 + 核心线）
- **退出检测**: Win32 Toolhelp32 枚举进程 + OpenProcess/WaitForSingleObject
- **状态桥接**: PowerShell hook 写入 `%TEMP%\claude-halo-state.txt`，Rust 150ms 轮询

---

## 📋 常见问题

<details>
<summary><b>光环没有出现？</b></summary>

- 确认插件已安装并启用：`/plugin` 查看列表
- 确认 Claude Code 版本支持插件系统
- 检查 `%TEMP%\claude-halo-state.txt` 是否存在，若存在但光环未出现可删除后重试
</details>

<details>
<summary><b>光环一直在某个颜色不变？</b></summary>

- 光环通过 Toolhelp32 检测 `claude.exe` 进程存活性来跟随退出
- 若 Claude Code 已退出但光环仍在，可右键 → 退出
</details>

<details>
<summary><b>光环挡住后面的窗口了？</b></summary>

- Ctrl+Shift+F12 切换点击穿透（15 秒后自动恢复）
- 或右键光环 → 点击穿透 切换
</details>

---

<p align="center">
  <sub>MIT License · Made with care by <a href="https://github.com/Houyusu">Houyu</a></sub>
</p>
