# One command: git pull + Rust tests + debug APK build/install on phone (+ optional PC host bundle).
#
# Usage:
#   .\scripts\update-all.ps1 -Serial RFCNC0PWD4E -Launch
#   .\scripts\update-all.ps1 -Serial RFCNC0PWD4E -SkipPull -SkipTests -Launch

param(
    [string]$Serial = "",
    [switch]$AllowDirty,
    [switch]$SkipPull,
    [switch]$SkipTests,
    [switch]$SkipPhoneApk,
    [switch]$BuildPcHostBundle,
    [switch]$EnableMdnsFirewall,
    [switch]$Launch
)

$ErrorActionPreference = "Stop"
$Root = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
Set-Location $Root

if (-not $SkipPull) {
    & (Join-Path $Root "scripts\update-windows.ps1") @(
        if ($AllowDirty) { "-AllowDirty" }
    )
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
}

if (-not $SkipTests) {
    Write-Host "Running cargo test --workspace ..." -ForegroundColor Cyan
    cargo +stable-x86_64-pc-windows-msvc test --workspace
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
}

if ($BuildPcHostBundle) {
    & (Join-Path $Root "scripts\build-pc-host-bundles.ps1")
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
}

if ($EnableMdnsFirewall) {
    Write-Host "Applying Windows mDNS / PC Host firewall rules (admin) ..." -ForegroundColor Cyan
    & (Join-Path $Root "scripts\enable-pc-host-mdns-windows.ps1")
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
}

if (-not $SkipPhoneApk) {
    $apkArgs = @()
    if ($Serial) { $apkArgs += "-Serial", $Serial }
    if ($AllowDirty) { $apkArgs += "-AllowDirty" }
    if ($SkipTests) { $apkArgs += "-SkipTests" }
    if ($Launch) { $apkArgs += "-Launch" }
    & (Join-Path $Root "scripts\update-phone-apk.ps1") @apkArgs
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
}

Write-Host ""
Write-Host "PASS update-all completed." -ForegroundColor Green
