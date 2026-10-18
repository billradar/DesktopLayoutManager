param([string]$OutputDirectory)

$ErrorActionPreference = 'Stop'
$root = $PSScriptRoot
$dist = if ([string]::IsNullOrWhiteSpace($OutputDirectory)) {
    Join-Path $root 'dist'
} else {
    [IO.Path]::GetFullPath($OutputDirectory)
}

if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    throw 'Rust/Cargo was not found. Install the stable Rust toolchain and reopen PowerShell.'
}
if (-not (Test-Path -LiteralPath (Join-Path $root 'assets\DesktopLayoutManager.ico'))) {
    throw 'Application icon assets\DesktopLayoutManager.ico is missing.'
}

Push-Location $root
try {
    cargo build --release --bin DesktopLayoutManager --lib
    if ($LASTEXITCODE -ne 0) { throw 'Native Win32 build failed.' }
    New-Item -ItemType Directory -Force -Path $dist | Out-Null
    Copy-Item (Join-Path $root 'target\release\DesktopLayoutManager.exe') $dist -Force
    Copy-Item (Join-Path $root 'target\release\desktop_layout_manager.dll') $dist -Force
    Write-Host "Build complete: $dist\DesktopLayoutManager.exe"
    Write-Host 'The ICO is embedded in the EXE; the Rust core DLL is shipped beside it.'
} finally {
    Pop-Location
}
