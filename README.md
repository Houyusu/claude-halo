<h1 align="center">
  Claude Halo
</h1>

<p align="center">
  <img src="https://img.shields.io/badge/platform-Windows-0078D6?style=flat-square&logo=windows" alt="Windows">
  <img src="https://img.shields.io/badge/macOS-即将上线-lightgrey?style=flat-square&logo=apple" alt="macOS coming soon">
  <img src="https://img.shields.io/badge/plugin-Claude%20Code-ff8830?style=flat-square" alt="Claude Code Plugin">
  <img src="https://img.shields.io/badge/license-MIT-33cc55?style=flat-square" alt="MIT">
</p>

---

## Claude Halo —— 给你的终端装一盏会思考的灯

灵感来源于《底特律：变人》。还记得康纳额头那一圈流转的黄蓝色光环吗？仿生人思考时，光环律动；做出决定时，光环变色。我一直想要那种感觉——不是盯着冷冰冰的日志，而是在余光里就能感知到 Claude 此刻的"心绪"。

Claude Halo 就是这样一个东西。它安静地待在屏幕右下角，不遮挡、不弹出、不打扰——鼠标能直接穿透它点到底下的窗口——但它的光环从不静止：

| 状态 | 颜色 | 动画 | |
|------|------|------|------|
| **待命** | `#aaaaaa` 灰白 | 慢速旋转，低存在感 | <img src="https://raw.githubusercontent.com/Houyusu/claude-halo/master/docs/states/idle.svg" width="32" height="32"> |
| **思考** | `#ff8830` 琥珀 | 呼吸辉光，一圈一圈 | <img src="https://raw.githubusercontent.com/Houyusu/claude-halo/master/docs/states/thinking.svg" width="32" height="32"> |
| **执行** | `#3399ff` 蓝色 | 高速旋转，工具调用密集时节奏更快 | <img src="https://raw.githubusercontent.com/Houyusu/claude-halo/master/docs/states/executing.svg" width="32" height="32"> |
| **等待输入** | `#ee3333` 红色 | 脉动闪烁——Permission 弹窗、确认提示，不会错过 | <img src="https://raw.githubusercontent.com/Houyusu/claude-halo/master/docs/states/input_needed.svg" width="32" height="32"> |
| **完成** | `#33cc55` 绿色 | 柔和呼吸，安静地告诉你好了 | <img src="https://raw.githubusercontent.com/Houyusu/claude-halo/master/docs/states/completed.svg" width="32" height="32"> |
| **压缩** | `#9944ff` 紫色 | 半径脉冲，上下文整理中，短暂又优雅 | <img src="https://raw.githubusercontent.com/Houyusu/claude-halo/master/docs/states/compacting.svg" width="32" height="32"> |

光环有 3 层渲染——外层的幽光晕影、中层的渐变过渡、内层的实色主线——加上弧段间的流体形变，视觉上既是"技术感的精确"，又是"手作感的柔和"。

👉 **[在线演示](https://houyusu.github.io/claude-halo/)**

---

## 安装

### Windows

在 Claude Code 中输入：

```bash
/plugin marketplace add Houyusu/claude-halo
/plugin install claude-halo-win@claude-halo
```

### Mac OS

即将上线 🚧

---

## 更新

```bash
/plugin update claude-halo-win
```

### 卸载

```bash
/plugin uninstall claude-halo-win
```

---

<p align="center">
  <sub>MIT License · Made with care by <a href="https://github.com/Houyusu">Houyu</a></sub>
</p>
