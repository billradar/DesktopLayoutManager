param([string]$OutputDirectory)

$ErrorActionPreference = 'Stop'
$project = Join-Path $PSScriptRoot 'ui\DesktopLayoutManager.WinUI.csproj'
$dist = if ([string]::IsNullOrWhiteSpace($OutputDirectory)) {
    Join-Path $PSScriptRoot 'dist'
} else {
    [IO.Path]::GetFullPath($OutputDirectory)
}
$icon = Join-Path $PSScriptRoot 'assets\DesktopLayoutManager.ico'

if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    throw 'Rust/Cargo was not found. Install the stable Rust toolchain and reopen PowerShell.'
}
if (-not (Get-Command dotnet -ErrorAction SilentlyContinue)) {
    throw 'The .NET SDK was not found. Install the .NET SDK required by this project and reopen PowerShell.'
}

New-Item -ItemType Directory -Force -Path $dist | Out-Null
& dotnet publish $project -c Release -r win-x64 --self-contained true -p:WindowsAppSDKSelfContained=false -o $dist
if ($LASTEXITCODE -ne 0) { throw 'WinUI 3 publish failed.' }
if (-not (Test-Path -LiteralPath (Join-Path $dist 'DesktopLayoutManager.exe'))) {
    throw 'The WinUI 3 executable was not produced.'
}
Copy-Item -LiteralPath $icon -Destination (Join-Path $dist 'DesktopLayoutManager.ico') -Force
Write-Host "Build complete: $(Join-Path $dist 'DesktopLayoutManager.exe')"
Write-Host 'The app includes the .NET runtime and prompts the user to acquire Windows App SDK Runtime if needed.'
