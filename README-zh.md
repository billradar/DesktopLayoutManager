# Desktop Layout Manager

[English](README.md) | 简体中文

[![CI — Native Win32 build](https://github.com/billradar/DesktopLayoutManager-rust/actions/workflows/native-win32.yml/badge.svg?branch=native-win32-ui)](https://github.com/billradar/DesktopLayoutManager-rust/actions/workflows/native-win32.yml)
[![CD — Release](https://github.com/billradar/DesktopLayoutManager-rust/actions/workflows/release.yml/badge.svg)](https://github.com/billradar/DesktopLayoutManager-rust/actions/workflows/release.yml)

**Desktop Layout Manager** 是一款面向 Windows 的桌面图标布局管理工具，使用 Rust 核心和原生 Win32 界面，帮助你查看桌面图标布局、预览空位补齐方案，并在需要时保存和恢复布局。

> 当前原生 Win32 界面仍在迁移完善中，部分界面体验可能与旧版不同。

## 功能

- **查看桌面布局**：读取桌面图标及其位置。
- **空位补齐**：预览整理方案，确认后执行图标补位。
- **布局快照**：保存当前布局，并从快照恢复。
- **撤销操作**：撤销最近一次补位操作。
- **数据导出**：将桌面布局导出为 CSV。
- **原生 Windows 界面**：采用 Win32 窗口、按钮和系统文件对话框。
- **Rust 技术栈**：界面与核心逻辑主要使用 Rust 实现。

## 下载与运行

请前往 [GitHub Actions](https://github.com/billradar/DesktopLayoutManager-rust/actions) 查看自动构建记录；正式版本发布后，可在 [Releases](https://github.com/billradar/DesktopLayoutManager-rust/releases) 获取对应文件。

Windows x64 Release 仅附带 `DesktopLayoutManager.exe` 和 `SHA256SUMS.txt`。GitHub 会自动提供源码 ZIP 和 TAR.GZ 压缩包，不额外上传应用 ZIP 或 DLL。

## 从源码构建

构建环境：Windows x64、Rust stable 工具链及 Windows SDK。

在项目根目录运行：

```powershell
.\build.ps1
```

也可以运行 `build.cmd`。构建脚本会生成应用程序文件。

## CI / CD 状态

- **CI（持续集成）**：代码推送或相关 PR 会触发 Windows x64 构建，并运行 Rust 核心测试。查看 [CI 工作流](https://github.com/billradar/DesktopLayoutManager-rust/actions/workflows/native-win32.yml)。
- **CD（发布）**：推送以 `v` 开头的版本标签（例如 `v0.1.0`）会触发构建和 GitHub Release；也可在 Actions 页面手动运行发布工作流并填写版本号。查看 [Release 工作流](https://github.com/billradar/DesktopLayoutManager-rust/actions/workflows/release.yml)。
- **发布文件**：仅上传 `DesktopLayoutManager.exe` 与 `SHA256SUMS.txt`；源码 ZIP 和 TAR.GZ 由 GitHub 自动提供。

## 项目状态

项目正在从 WinUI 3 / XAML 界面迁移到原生 Win32 界面。当前已提供桌面读取、补位预览与执行、布局快照保存和恢复、撤销以及 CSV 导出等主要操作。列表区域仍在完善中。

## 许可证

本项目使用 MIT License，详见 [LICENSE](LICENSE)。
