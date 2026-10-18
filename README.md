# DesktopLayoutManager (WinUI 3 + Rust)

桌面布局管理器的 Windows GUI 使用 WinUI 3 与 Fluent 控件；桌面 Explorer 读取、快捷方式分类、补位规划、恢复、自动备份和失败回滚由 Rust 本地库负责。应用不提供 CLI。

## 构建

在 Windows x64 上安装 Rust stable、.NET SDK 8 或更新版本，然后在项目目录运行：

```powershell
.\build.ps1
```

或使用 `build.cmd`。应用发布到 `dist\DesktopLayoutManager.exe`，并带上 Rust 核心 DLL 与 .NET 运行时。当前发布目录约 155 MB，应用需要从 `dist` 目录运行。Windows App SDK 使用框架依赖部署：首次启动会自动检测匹配的 Windows App SDK Runtime；如果缺失，Windows App SDK bootstrapper 会提示用户获取并安装运行时。安装后重新启动应用即可。

要生成 Windows 安装包，在项目目录运行：

```powershell
.\build-installer.ps1
```

脚本会生成 `dist\installer\DesktopLayoutManager-Setup.exe` 和 `dist\installer\DesktopLayoutManager.msi`。建议用 EXE 向导安装：它检测并按需安装 Microsoft Visual C++ Redistributable，再调用 Windows App SDK Runtime 2.5.1 安装器，最后安装 MSI。Runtime 安装器会自行检查所需组件并在已满足时跳过部署。单独运行 MSI 不会执行外层依赖安装流程。

## 界面操作

- 主界面提供“仅快照”“仅补位”“快照 + 补位”三种可选模式。
- 中央表格显示桌面项目、状态、当前位置和目标/快照位置。
- 支持读取桌面、保存/打开快照、预览差异、恢复布局、预览补位、执行补位、撤销本次补位和导出 JSON/CSV。
- 执行补位或恢复前会显示确认；执行前自动保存当前布局备份。

## 安全约束

- 补位按列优先排列：每列从上到下，再从左到右。
- 只移动能与当前用户或公共桌面目录中的 `.lnk` / `.url` 快捷方式可靠匹配的项目。
- 真实文件、文件夹、系统图标和未识别项目不会移动；无法可靠分类、推断网格或检测到多显示器时，补位会停止。
- 执行确认后会重新读取桌面并重算计划；桌面发生变化时会取消操作。
- Explorer 拒绝移动或位置验证失败时，已完成的移动会尽量回滚；自动备份保存在 `%LOCALAPPDATA%\DesktopLayoutManager\Backups`。
- 自动排列启用时不执行补位。Explorer ListView 消息可能受 Windows 更新、权限级别和 Explorer 重启影响；首次使用建议先在可恢复的桌面布局上验证。

## 项目结构

- `ui/`：WinUI 3 Fluent GUI、文件选择器和交互流程。
- `src/lib.rs`：Rust 桌面读取、分类、快照、预览、补位、恢复、备份和回滚核心，以及供 GUI 调用的 JSON C ABI。
- `assets/`：应用图标与预览图。
- `build.ps1`、`build.cmd`：生成可分发的 Windows x64 GUI 应用。
