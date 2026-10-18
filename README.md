# DesktopLayoutManager (Native Win32 + Rust)

桌面布局管理器正在从 WinUI 3 / XAML 迁移到原生 Win32 界面。桌面读取、快捷方式分类、补位规划、恢复、自动备份和失败回滚仍由现有 Rust 核心负责。

## 构建

在 Windows x64 上安装 Rust stable 工具链与 Windows SDK（用于编译 Windows 资源），在项目根目录运行：

```powershell
.\build.ps1
```

或运行 `build.cmd`。输出位于 `dist\DesktopLayoutManager.exe`，Rust 核心 DLL 位于同目录。应用图标通过资源编译阶段嵌入 EXE，不需要单独分发 ICO 文件。

## 当前原生 Win32 迁移状态

- 已加入 Win32 窗口、原生按钮和只读多行输出区。
- 已接入桌面读取、补位预览与执行补位的 Rust 核心操作。
- 已加入原生文件对话框，支持保存/打开快照、恢复布局、撤销最近一次补位以及 CSV 导出。
- 补位执行要求先预览；核心会再次校验桌面快照与移动计划，并在执行前自动备份。
- 当前列表仍使用原生多行文本控件，尚未替换为 ListView；迁移完成前不要把此分支视为功能等同于旧版的正式发布版本。

## 安全约束

- 仅移动能够可靠匹配的快捷方式；真实文件、文件夹、系统图标和未识别项目不会移动。
- 无法可靠分类、推断网格或检测到多显示器时，补位会停止。
- 确认执行前重新读取桌面并重算计划；桌面发生变化时取消操作。
- 执行前自动备份；移动失败时尽量回滚。
- 自动排列启用时不执行补位。

## 项目结构

- `src/main.rs`：原生 Win32 GUI。
- `src/lib.rs`：Rust 桌面操作核心及 JSON C ABI。
- `assets/DesktopLayoutManager.ico`：编译进 EXE 的应用图标源文件。
- `build.rs`：Windows 资源编译步骤。
