# Claude Halo

桌面光环指示器——在屏幕右下角显示一个彩色光环，实时反映 Claude Code 的运行状态。

> 🪟 Windows 专属。需要已安装 Claude Code。

## 效果

光环颜色随 Claude Code 状态自动变化：

| 颜色 | 状态 | 含义 |
|------|------|------|
| 灰色 | 待命 | 等待你的输入 |
| 橙色 | 思考 | Claude 正在推理 |
| 蓝色 | 执行 | 正在调用工具（读写文件、运行命令等） |
| 红色 | 等待输入 | Claude 需要你回答问题或确认操作 |
| 绿色 | 完成 | 任务完成 |
| 紫色 | 压缩 | 正在整理上下文 |

## 安装

在 Claude Code 中输入：

```
/plugin marketplace add Houyusu/claude-halo
/plugin install claude-halo@claude-halo
```

安装后，下次启动 Claude Code 时光环自动出现。

## 卸载

```
/plugin uninstall claude-halo
```

## 操作

- **右键光环** → 打开菜单（切换点击穿透、退出）
- **Ctrl+Shift+F12** → 快速切换点击穿透（让鼠标穿过光环操作后面的窗口）

## 系统要求

- Windows 10 / 11
- Claude Code（通过 npm 安装）

## 常见问题

**光环没有出现？**
- 确认已安装并启用：`/plugin` 查看插件列表
- 确认 Claude Code 版本足够新（支持插件系统）
- 检查 `%TEMP%\claude-halo-state.txt` 是否存在，若存在可删除后重试

**光环一直在某个颜色不变？**
- 光环通过检测 claude.exe 进程存活来跟随退出，若进程已退出但光环仍在，可右键 → 退出

**可以调整光环位置或大小吗？**
- 光环固定在屏幕右下角，大小不可调。这是为了保持视觉一致性和低干扰。
