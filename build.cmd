@echo off
setlocal
cd /d "%~dp0"
where cargo >nul 2>nul
if errorlevel 1 (
  echo Rust/Cargo was not found. Install the stable Rust toolchain and reopen Command Prompt.
  exit /b 1
)
if not exist "assets\DesktopLayoutManager.ico" (
  echo Missing assets\DesktopLayoutManager.ico
  exit /b 1
)
cargo build --release --bin DesktopLayoutManager --lib
if errorlevel 1 exit /b 1
if not exist "dist" mkdir "dist"
copy /y "target\release\DesktopLayoutManager.exe" "dist\DesktopLayoutManager.exe" >nul
if errorlevel 1 exit /b 1
copy /y "target\release\desktop_layout_manager.dll" "dist\desktop_layout_manager.dll" >nul
if errorlevel 1 exit /b 1
echo Build complete: dist\DesktopLayoutManager.exe
echo The ICO is embedded in the EXE; the Rust core DLL is shipped beside it.
