# Desktop Layout Manager

English | [简体中文](README-zh.md)

[![CI — Native Win32 build](https://github.com/billradar/DesktopLayoutManager-rust/actions/workflows/native-win32.yml/badge.svg?branch=native-win32-ui)](https://github.com/billradar/DesktopLayoutManager-rust/actions/workflows/native-win32.yml)
[![CD — Release](https://github.com/billradar/DesktopLayoutManager-rust/actions/workflows/release.yml/badge.svg)](https://github.com/billradar/DesktopLayoutManager-rust/actions/workflows/release.yml)

**Desktop Layout Manager** is a Windows desktop icon layout tool built with a Rust core and a native Win32 interface. It helps you inspect desktop icon positions, preview how empty spaces can be filled, and save or restore layouts when needed.

> The native Win32 interface is still being migrated and refined, so some parts of the experience may differ from earlier versions.

## Features

- **Inspect desktop layout** — read desktop icons and their positions.
- **Fill empty spaces** — preview an arrangement and apply it when ready.
- **Layout snapshots** — save the current layout and restore it later.
- **Undo** — revert the most recent fill operation.
- **Export data** — export desktop layout information to CSV.
- **Native Windows interface** — Win32 windows, buttons, and system file dialogs.
- **Rust-based implementation** — the interface and core logic are primarily written in Rust.

## Download and run

Visit [GitHub Actions](https://github.com/billradar/DesktopLayoutManager-rust/actions) for automated build artifacts. Once a version is published, downloads will be available on the [Releases](https://github.com/billradar/DesktopLayoutManager-rust/releases) page.

The Windows x64 release provides `DesktopLayoutManager.exe` and `SHA256SUMS.txt` as release assets. GitHub also provides source code archives (ZIP and TAR.GZ) automatically. No separate application ZIP or DLL is attached.

## Build from source

Requirements: Windows x64, the Rust stable toolchain, and the Windows SDK.

From the project root, run:

```powershell
.\build.ps1
```

You can also run `build.cmd`. The build script generates the application binaries.

## CI / CD status

- **CI (continuous integration):** pushes and relevant pull requests trigger a Windows x64 build and Rust core tests. See the [CI workflow](https://github.com/billradar/DesktopLayoutManager-rust/actions/workflows/native-win32.yml).
- **CD (release):** pushing a version tag beginning with `v` (for example, `v0.1.0`) triggers a build and GitHub Release. You can also start the release workflow manually from Actions and provide a version. See the [release workflow](https://github.com/billradar/DesktopLayoutManager-rust/actions/workflows/release.yml).
- **Release assets:** `DesktopLayoutManager.exe` and `SHA256SUMS.txt`. Source code ZIP and TAR.GZ archives are provided automatically by GitHub.

## Project status

The project is migrating from a WinUI 3 / XAML interface to native Win32. Core workflows include reading desktop layout, previewing and applying space-filling, saving and restoring snapshots, undoing the last operation, and exporting CSV. The list area is still being refined.

## License

This project is licensed under the MIT License. See [LICENSE](LICENSE).
