<h1 align="center">
  Claude Halo
</h1>

<p align="center">
  <img src="https://img.shields.io/badge/platform-Windows-0078D6?style=flat-square&logo=windows" alt="Windows">
  <img src="https://img.shields.io/badge/platform-macOS-lightgrey?style=flat-square&logo=apple" alt="macOS">
  <img src="https://img.shields.io/badge/plugin-Claude%20Code-ff8830?style=flat-square" alt="Claude Code Plugin">
  <img src="https://img.shields.io/badge/license-MIT-33cc55?style=flat-square" alt="MIT">
</p>

---

## 六种状态

光环颜色和动画随 Claude Code 状态自动变化，每个状态有独特的视觉节奏。

完整动画效果请访问 **[在线演示](https://houyusu.github.io/claude-halo/)**。

<table>
<tr>
  <td align="center" width="120px">
    <img src="https://raw.githubusercontent.com/Houyusu/claude-halo/master/docs/states/idle.svg" width="48" height="48" alt="idle"/><br>
    <code style="color:#aaaaaa">#aaaaaa</code><br><b>待命</b><br><sub>等待你的输入</sub>
  </td>
  <td align="center" width="120px">
    <img src="https://raw.githubusercontent.com/Houyusu/claude-halo/master/docs/states/thinking.svg" width="48" height="48" alt="thinking"/><br>
    <code style="color:#ff8830">#ff8830</code><br><b>思考</b><br><sub>正在推理</sub>
  </td>
  <td align="center" width="120px">
    <img src="https://raw.githubusercontent.com/Houyusu/claude-halo/master/docs/states/executing.svg" width="48" height="48" alt="executing"/><br>
    <code style="color:#3399ff">#3399ff</code><br><b>执行</b><br><sub>正在调用工具</sub>
  </td>
  <td align="center" width="120px">
    <img src="https://raw.githubusercontent.com/Houyusu/claude-halo/master/docs/states/input_needed.svg" width="48" height="48" alt="input_needed"/><br>
    <code style="color:#ee3333">#ee3333</code><br><b>等待输入</b><br><sub>需要你的响应</sub>
  </td>
  <td align="center" width="120px">
    <img src="https://raw.githubusercontent.com/Houyusu/claude-halo/master/docs/states/completed.svg" width="48" height="48" alt="completed"/><br>
    <code style="color:#33cc55">#33cc55</code><br><b>完成</b><br><sub>任务执行完毕</sub>
  </td>
  <td align="center" width="120px">
    <img src="https://raw.githubusercontent.com/Houyusu/claude-halo/master/docs/states/compacting.svg" width="48" height="48" alt="compacting"/><br>
    <code style="color:#9944ff">#9944ff</code><br><b>压缩</b><br><sub>正在整理上下文</sub>
  </td>
</tr>
</table>

---

## 安装

### Windows

在 Claude Code 中输入：

```bash
/plugin marketplace add Houyusu/claude-halo
/plugin install claude-halo-win@claude-halo
```

### macOS

```bash
/plugin marketplace add Houyusu/claude-halo
/plugin install claude-halo-mac@claude-halo
```

安装后重新启动 Claude Code，光环自动出现在屏幕右下角。

### 从源码构建

```bash
# Windows
cd win/src-tauri && cargo build --release
# 二进制在 win/src-tauri/target/release/claude-halo.exe

# macOS
cd mac/src-tauri && cargo build --release
# 二进制在 mac/src-tauri/target/release/claude-halo
```

### 卸载

```bash
/plugin uninstall claude-halo-win   # Windows
/plugin uninstall claude-halo-mac   # macOS
```

---

<p align="center">
  <sub>MIT License · Made with care by <a href="https://github.com/Houyusu">Houyu</a></sub>
</p>
