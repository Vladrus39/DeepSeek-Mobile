# Download Termux + optional bootstrap on PC, install on device via ADB (phone needs no internet).
# Usage: . .\tools\android\env.ps1; .\scripts\install-termux-offline.ps1 [-Serial RFCNC0PWD4E]

param(
    [string]$Serial = ""
)

$ErrorActionPreference = "Stop"
$ProjectRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$ThirdParty = Join-Path $ProjectRoot "tools\android\third-party"
$TermuxApk = Join-Path $ThirdParty "termux-app_universal.apk"
$BootstrapZip = Join-Path $ThirdParty "bootstrap-aarch64.zip"

$adb = Join-Path $ProjectRoot "tools\android\sdk\platform-tools\adb.exe"
if (-not (Test-Path $adb)) {
    $fallback = Join-Path $env:LOCALAPPDATA "Android\Sdk\platform-tools\adb.exe"
    if (Test-Path $fallback) { $adb = $fallback } else { $adb = "adb" }
}

function Invoke-Adb {
    param([string[]]$AdbArgs)
    if ($Serial) { return & $adb -s $Serial @AdbArgs }
    return & $adb @AdbArgs
}

$devices = Invoke-Adb @("devices")
Write-Host $devices
if ($devices -match "unauthorized") {
    throw "ADB device is unauthorized. Unlock the phone and accept the USB debugging/RSA prompt."
}
if ($devices -notmatch "device`r?`n|device$") {
    throw "No authorized Android device found."
}

New-Item -ItemType Directory -Force -Path $ThirdParty | Out-Null

if (-not (Test-Path $TermuxApk) -or (Get-Item $TermuxApk).Length -lt 50MB) {
    Write-Host "Downloading Termux APK from F-Droid (com.termux_1002)..." -ForegroundColor Cyan
    curl.exe -fL --retry 3 -o $TermuxApk "https://f-droid.org/repo/com.termux_1002.apk"
}

Write-Host "Installing Termux APK..." -ForegroundColor Cyan
Invoke-Adb @("install", "-r", $TermuxApk)

if (-not (Test-Path $BootstrapZip) -or (Get-Item $BootstrapZip).Length -lt 5MB) {
    Write-Host "Downloading bootstrap-aarch64.zip (fallback if embedded bootstrap fails)..." -ForegroundColor Cyan
    curl.exe -fL --retry 3 -o $BootstrapZip `
        "https://github.com/termux/termux-packages/releases/download/bootstrap-2026.05.24-r1%2Bapt.android-7/bootstrap-aarch64.zip"
}

Write-Host "Pushing bootstrap zip to /sdcard/Download/ (optional fallback)..." -ForegroundColor Cyan
Invoke-Adb @("push", $BootstrapZip, "/sdcard/Download/bootstrap-aarch64.zip")

# These may be denied on newer Android versions; Termux can still complete first-run setup.
Invoke-Adb @("shell", "pm", "grant", "com.termux", "android.permission.READ_EXTERNAL_STORAGE") 2>$null | Out-Null
Invoke-Adb @("shell", "pm", "grant", "com.termux", "android.permission.WRITE_EXTERNAL_STORAGE") 2>$null | Out-Null

Write-Host "Starting Termux (bootstrap is embedded in APK; works offline)..." -ForegroundColor Cyan
Invoke-Adb @("shell", "am", "start", "-n", "com.termux/.app.TermuxActivity")

Write-Host ""
Write-Host "Termux installed. On the phone (no internet needed):" -ForegroundColor Green
Write-Host "  1. Wait until Termux finishes first-time setup (1-2 min)."
Write-Host "  2. If it asks for network, choose install from local file:"
Write-Host "     Downloads / bootstrap-aarch64.zip"
Write-Host "  3. In Termux shell run:"
Write-Host "       mkdir -p ~/deepseek-project"
Write-Host "       mkdir -p ~/.termux"
Write-Host "       echo allow-external-apps=true >> ~/.termux/termux.properties"
Write-Host "       termux-reload-settings"
Write-Host "  4. Force-stop Termux, reopen DeepSeek Mobile, grant RUN_COMMAND if Android asks."
