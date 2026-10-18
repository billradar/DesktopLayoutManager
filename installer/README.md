# Windows installer

DesktopLayoutManager-Setup.exe is the recommended installer. It runs a WiX Burn setup wizard and chains:

1. Microsoft Visual C++ Redistributable (x64), only when missing or older than the bundled version.
2. DesktopLayoutManager.msi.

The Visual C++ runtime remains installed when the application is uninstalled. The standalone MSI is also produced for managed deployment; use the EXE when installing interactively so the prerequisite is handled first.

## Build

From the repository root, run ./build-installer.ps1. The script builds the native Rust Win32 application, downloads the Microsoft-signed x64 Visual C++ prerequisite into the ignored work/installer/payloads folder when absent, verifies its Authenticode signature, then builds the MSI and Burn bundle into dist/installer.

The installer build uses the local wix .NET tool pinned in .config/dotnet-tools.json and the matching WiX Burn extensions. The application itself does not require the .NET SDK or Windows App SDK Runtime; .NET is used only by the installer build tooling. Internet access is needed for the first installer build to restore WiX and obtain the prerequisite installer.
