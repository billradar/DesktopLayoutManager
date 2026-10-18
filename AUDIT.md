# Desktop Layout Manager 实现与构建记录

日期：2026-10-18

## 本次实现

- 将桌面界面迁移为 C# / WinUI 3，使用 Fluent 控件、Mica 背景、三种可切换模式、文件选择器、确认对话框和四列预览表。
- 保留 Rust 中的 Explorer ListView 读取、快捷方式分类、列优先补位、快照恢复、执行前备份和失败回滚逻辑；通过 JSON C ABI 供 WinUI 调用。
- 删除旧的 Rust Win32 窗口入口，Rust crate 现在只输出原生操作 DLL，应用仅有 GUI。
- 配置 Windows App SDK unpackaged auto-initializer。启动时查找匹配运行时；找不到时启用官方 `OnNoMatch_ShowUI` 获取提示。应用自带 .NET 运行时，避免要求用户另装 .NET。
- WinUI 应用图标仍从 `assets\DesktopLayoutManager.ico` 编译进 EXE，并在发布目录保留 ICO 侧文件。

## 验证

- `cargo check --lib`：通过，无 Rust 编译警告。
- `cargo build --release --lib`：通过。
- `cargo test --lib`：9 项通过。
- `cargo fmt --all -- --check`：通过。
- `dotnet build ui\DesktopLayoutManager.WinUI.csproj -c Debug`：通过。
- `dotnet publish ui\DesktopLayoutManager.WinUI.csproj -c Release -r win-x64 --self-contained true -p:WindowsAppSDKSelfContained=false -o dist`：通过。
- 发布输出包含 WinUI EXE、Rust DLL、.NET 自包含运行时和 Windows App SDK bootstrapper。

## 运行验证边界

- 构建机已安装 Windows App Runtime 2.5.1，因此“缺少运行时”时的首次启动提示没有在此机器上触发验证；项目已配置官方自动检测与获取提示。
- 没有执行会移动桌面图标的补位、恢复或撤销操作。
- 发布采用文件夹部署；当前整个 `dist` 文件夹约 155 MB。它自带 .NET，仍需 Windows App SDK Runtime 2.5.1；缺少时由 bootstrapper 提示获取。
