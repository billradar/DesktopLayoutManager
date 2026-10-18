@echo off
setlocal
cd /d "%~dp0"

where cargo >nul 2>nul
if errorlevel 1 (
    echo Rust/Cargo was not found. Install the stable Rust toolchain and reopen Command Prompt.
    exit /b 1
)
where dotnet >nul 2>nul
if errorlevel 1 (
    echo The .NET SDK was not found. Install the .NET SDK required by the project and reopen Command Prompt.
    exit /b 1
)

dotnet publish "ui\DesktopLayoutManager.WinUI.csproj" -c Release -r win-x64 --self-contained true -p:WindowsAppSDKSelfContained=false -o "dist"
if errorlevel 1 exit /b 1

copy /y "assets\DesktopLayoutManager.ico" "dist\DesktopLayoutManager.ico" >nul
if errorlevel 1 exit /b 1

echo Build complete: dist\DesktopLayoutManager.exe
echo The Windows App SDK runtime must be present; if missing, the app will guide the user to acquire it.
