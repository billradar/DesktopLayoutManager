$ErrorActionPreference = 'Stop'

$root = $PSScriptRoot
$version = '1.0.0'
$distDir = Join-Path $root 'dist'
$publishDir = Join-Path $distDir 'app'
$installerDir = Join-Path $distDir 'installer'
$payloadDir = Join-Path $root 'work\installer\payloads'
$wixExtension = 'WixToolset.BootstrapperApplications.wixext/6.0.2'
$wixUtilExtension = 'WixToolset.Util.wixext/6.0.2'

function Invoke-Checked {
    param([Parameter(Mandatory)][string]$File, [Parameter(Mandatory)][string[]]$Arguments)
    & $File @Arguments
    if ($LASTEXITCODE -ne 0) { throw "Command failed ($LASTEXITCODE): $File $($Arguments -join ' ')" }
}

function Get-MicrosoftInstaller {
    param([Parameter(Mandatory)][string]$Url, [Parameter(Mandatory)][string]$Path)
    if (-not (Test-Path -LiteralPath $Path)) {
        Write-Host "Downloading $([IO.Path]::GetFileName($Path)) from Microsoft..."
        Invoke-WebRequest -Uri $Url -OutFile $Path
    }
    $signature = Get-AuthenticodeSignature -FilePath $Path
    if ($signature.Status -ne 'Valid' -or $signature.SignerCertificate.Subject -notmatch 'Microsoft Corporation') {
        throw "The downloaded prerequisite does not have a valid Microsoft signature: $Path"
    }
}

if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    throw 'Rust/Cargo was not found. Install the stable Rust toolchain and reopen PowerShell.'
}
if (-not (Get-Command dotnet -ErrorAction SilentlyContinue)) {
    throw 'The .NET SDK was not found. Install the .NET SDK required by this project and reopen PowerShell.'
}

New-Item -ItemType Directory -Force -Path $installerDir, $payloadDir | Out-Null

if (Test-Path -LiteralPath $publishDir) {
    $resolvedDist = (Resolve-Path -LiteralPath $distDir).Path.TrimEnd('\')
    $resolvedPublish = (Resolve-Path -LiteralPath $publishDir).Path.TrimEnd('\')
    if ($resolvedPublish -ne (Join-Path $resolvedDist 'app')) {
        throw "Refusing to clean an unexpected publish directory: $resolvedPublish"
    }
    Remove-Item -LiteralPath $resolvedPublish -Recurse -Force
}

& (Join-Path $root 'build.ps1') -OutputDirectory $publishDir
if ($LASTEXITCODE -ne 0) { throw 'The application publish failed.' }

$runtimeInstaller = Join-Path $payloadDir 'WindowsAppRuntimeInstall-x64.exe'
$vcRedistInstaller = Join-Path $payloadDir 'vc_redist.x64.exe'
Get-MicrosoftInstaller `
    -Url 'https://download.microsoft.com/download/de664922-b046-432f-bb2a-54aec3c5f1f6/WindowsAppRuntimeInstall-x64.exe' `
    -Path $runtimeInstaller
Get-MicrosoftInstaller `
    -Url 'https://aka.ms/vs/17/release/vc_redist.x64.exe' `
    -Path $vcRedistInstaller
$vcRedistVersion = [version](Get-Item -LiteralPath $vcRedistInstaller).VersionInfo.FileVersion
$vcRedistDetectCondition = "VcRedistInstalledNumber = 1 AND (VcRedistMajorNumber > $($vcRedistVersion.Major) OR (VcRedistMajorNumber = $($vcRedistVersion.Major) AND (VcRedistMinorNumber > $($vcRedistVersion.Minor) OR (VcRedistMinorNumber = $($vcRedistVersion.Minor) AND (VcRedistBuildNumber > $($vcRedistVersion.Build) OR (VcRedistBuildNumber = $($vcRedistVersion.Build) AND VcRedistRevisionNumber >= $($vcRedistVersion.Revision)))))))"

Invoke-Checked dotnet @('tool', 'restore', '--tool-manifest', (Join-Path $root '.config\dotnet-tools.json'))
Push-Location $root
try {
    $extensionDll = Join-Path $env:USERPROFILE '.wix\extensions\WixToolset.BootstrapperApplications.wixext\6.0.2\wixext6\WixToolset.BootstrapperApplications.wixext.dll'
    if (-not (Test-Path -LiteralPath $extensionDll)) {
        Invoke-Checked dotnet @('tool', 'run', 'wix', 'extension', 'add', '-g', $wixExtension)
    }
    $utilExtensionDll = Join-Path $env:USERPROFILE '.wix\extensions\WixToolset.Util.wixext\6.0.2\wixext6\WixToolset.Util.wixext.dll'
    if (-not (Test-Path -LiteralPath $utilExtensionDll)) {
        Invoke-Checked dotnet @('tool', 'run', 'wix', 'extension', 'add', '-g', $wixUtilExtension)
    }

    Invoke-Checked dotnet @(
        'tool', 'run', 'wix', 'build',
        (Join-Path $root 'installer\DesktopLayoutManager.Product.wxs'),
        '-arch', 'x64',
        '-d', "ProductVersion=$version",
        '-bindpath', "repo=$root",
        '-bindpath', "publish=$publishDir",
        '-o', (Join-Path $installerDir 'DesktopLayoutManager.msi')
    )

    Invoke-Checked dotnet @(
        'tool', 'run', 'wix', 'build',
        (Join-Path $root 'installer\DesktopLayoutManager.Bundle.wxs'),
        '-arch', 'x64',
        '-ext', 'WixToolset.BootstrapperApplications.wixext',
        '-ext', 'WixToolset.Util.wixext',
        '-d', "ProductVersion=$version",
        '-d', "VcRedistDetectCondition=$vcRedistDetectCondition",
        '-bindpath', "repo=$root",
        '-bindpath', "payloads=$payloadDir",
        '-bindpath', "msi=$installerDir",
        '-o', (Join-Path $installerDir 'DesktopLayoutManager-Setup.exe')
    )
}
finally {
    Pop-Location
}

Write-Host 'Installer build complete:'
Write-Host (Join-Path $installerDir 'DesktopLayoutManager-Setup.exe')
Write-Host (Join-Path $installerDir 'DesktopLayoutManager.msi')
