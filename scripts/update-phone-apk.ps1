# Build the debug Android APK from the current checkout and install it on a USB device.
# App data under files/deepseek-mobile/ is preserved (adb install -r).
#
# First-time PC + repo setup: docs/INSTALL_UPDATE.md or scripts/setup-pc-windows.ps1
# Android SDK slice: tools/android/ (see tools/android/README.md)
#
# Usage (from repo root):
#   . .\tools\android\env.ps1
#   .\scripts\update-phone-apk.ps1 -Serial RFCNC0PWD4E
#
# Update source + APK in one go:
#   .\scripts\update-phone-apk.ps1 -Serial RFCNC0PWD4E -Pull

param(
    [string]$Serial = "",
    [switch]$Pull,
    [switch]$AllowDirty,
    [switch]$SkipTests,
    [switch]$SkipBuild,
    [switch]$Launch
)

$ErrorActionPreference = "Stop"
$ProjectRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$ApkRel = "target\dx\deepseek-mobile\debug\android\app\app\build\outputs\apk\debug\app-debug.apk"
$ApkPath = Join-Path $ProjectRoot $ApkRel
$Package = "com.deepseek.mobile"
$Activity = "$Package/dev.dioxus.main.MainActivity"

function Resolve-Adb {
    $local = Join-Path $ProjectRoot "tools\android\sdk\platform-tools\adb.exe"
    if (Test-Path $local) { return $local }
    return "adb"
}

function Invoke-Adb([string[]]$AdbArgs) {
    if ($Serial) {
        return & $script:AdbExe -s $Serial @AdbArgs 2>&1
    }
    return & $script:AdbExe @AdbArgs 2>&1
}

Set-Location $ProjectRoot
$script:AdbExe = Resolve-Adb

if ($Pull) {
    Write-Host "Updating git checkout (fast-forward only)..." -ForegroundColor Cyan
    & (Join-Path $ProjectRoot "scripts\update-windows.ps1") @(
        if ($AllowDirty) { "-AllowDirty" }
    )
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
}

$envScript = Join-Path $ProjectRoot "tools\android\env.ps1"
if (-not (Test-Path $envScript)) {
    throw "Missing $envScript - run scripts/setup-android-offline.ps1 or copy SDK into tools/android/sdk"
}
. $envScript

if (-not $SkipTests) {
    Write-Host "Running cargo test --workspace ..." -ForegroundColor Cyan
    cargo +stable-x86_64-pc-windows-msvc test --workspace
    if ($LASTEXITCODE -ne 0) { throw "cargo test failed" }
}

if (-not $SkipBuild) {
    Write-Host "Building debug APK (dx) ..." -ForegroundColor Cyan
    if ($Serial) {
        dx build --android --package deepseek-mobile --device $Serial --verbose
    } else {
        dx build --android --package deepseek-mobile --verbose
    }
    if ($LASTEXITCODE -ne 0) { throw "dx build failed" }
}

if (-not (Test-Path $ApkPath)) {
    throw "APK not found: $ApkPath`nBuild first without -SkipBuild."
}

$dev = (Invoke-Adb @("devices") | Out-String)
if ($dev -notmatch "`tdevice") {
    throw "No adb device in 'device' state. Connect USB debugging and run: adb devices"
}

Write-Host "Installing $ApkRel ..." -ForegroundColor Cyan
Invoke-Adb @("install", "-r", $ApkPath) | Out-Host
if ($LASTEXITCODE -ne 0) { throw "adb install failed" }

if ($Launch) {
    Write-Host "Launching app ..." -ForegroundColor Cyan
    Invoke-Adb @("shell", "am", "start", "-n", $Activity) | Out-Null
}

Write-Host ""
Write-Host "PASS Phone APK updated." -ForegroundColor Green
Write-Host "  APK: $ApkPath"
Write-Host "  Package: $Package (user data kept on upgrade)"
Write-Host ""
Write-Host "Next: open the app, complete Termux setup if needed - docs/DEVICE_SETUP.md"
