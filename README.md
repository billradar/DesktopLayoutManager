# Desktop Layout Manager

English | [简体中文](README-zh.md)

**Desktop Layout Manager** is a Windows utility for organizing desktop icons. It helps you inspect the current icon layout, fill gaps left by removed icons, and save or restore arrangements without manually repositioning every icon.

## What it does

- **Inspect the desktop** — view desktop icons and their current positions.
- **Fill gaps** — preview how icons can be compacted into empty spaces, then apply the arrangement when it looks right.
- **Save and restore layouts** — create a snapshot of the current arrangement and restore it later.
- **Undo a fill operation** — revert the most recent gap-filling action.
- **Export layout data** — export icon layout information to CSV.

The app is built in Rust and uses a native Win32 interface. The Win32 UI is still being refined.

## Download

See [GitHub Releases](https://github.com/billradar/DesktopLayoutManager-rust/releases) for published Windows x64 builds. A release includes `DesktopLayoutManager.exe` and `SHA256SUMS.txt`.

## Build from source

Requirements: Windows x64, the Rust stable toolchain, and the Windows SDK.

From the project root, run:

```powershell
.\build.ps1
```

Alternatively, run `build.cmd`.

## License

MIT — see [LICENSE](LICENSE).
