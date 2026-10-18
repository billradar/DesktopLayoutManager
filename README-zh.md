# Desktop Layout Manager

[English](README.md) | 简体中文

**Desktop Layout Manager** 是一款用于整理 Windows 桌面图标的工具。它可以查看当前图标布局、填补删除图标后留下的空位，并保存或恢复布局，减少手动逐个拖动图标的麻烦。

## 主要功能

- **查看桌面布局**：查看桌面图标及其当前位置。
- **空位补齐**：预览图标如何移动以填补空位，确认效果后再应用布局。
- **保存与恢复布局**：保存当前图标排列，并在需要时恢复。
- **撤销补位**：撤销最近一次空位补齐操作。
- **导出布局数据**：将图标布局信息导出为 CSV。

项目使用 Rust 开发，并采用原生 Win32 界面。目前 Win32 界面仍在持续完善中。

## 下载

已发布的 Windows x64 版本请查看 [GitHub Releases](https://github.com/billradar/DesktopLayoutManager-rust/releases)。每个版本包含 `DesktopLayoutManager.exe` 和 `SHA256SUMS.txt`。

## 从源码构建

构建环境：Windows x64、Rust stable 工具链及 Windows SDK。

在项目根目录运行：

```powershell
.\build.ps1
```

也可以运行 `build.cmd`。

## 许可证

MIT，详见 [LICENSE](LICENSE)。
