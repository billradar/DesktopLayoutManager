# Windows installer

`DesktopLayoutManager-Setup.exe` is the recommended installer. It runs a WiX Burn setup wizard and chains these packages in order:

1. Microsoft Visual C++ Redistributable (x64), only when missing or older than the bundled version.
2. Windows App SDK Runtime 2.5.1 (x64), which is the runtime version used by the WinUI project. Its official installer checks/deploys the runtime packages and is safe to run when they are already installed.
3. `DesktopLayoutManager.msi`.

The two shared runtimes remain installed when the application is uninstalled. The standalone MSI is also produced for managed deployment; use the EXE when installing interactively so prerequisites are handled first.

## Build

From the repository root, run `./build-installer.ps1`. The script publishes the self-contained .NET app, downloads the two Microsoft-signed x64 prerequisite installers into the ignored `work/installer/payloads` folder when absent, verifies their Authenticode signatures, then builds the MSI and Burn bundle into `dist/installer`.

The build uses the local `wix` .NET tool pinned in `.config/dotnet-tools.json` and the matching WiX Burn extension. Internet access is needed for the first build to restore WiX and obtain the prerequisite installers.
