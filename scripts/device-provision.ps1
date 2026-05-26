# Provision DeepSeek-Mobile on a connected debug device (ADB).
# Does everything that does not require tapping the phone UI.
#
# Usage: . .\tools\android\env.ps1; .\scripts\device-provision.ps1 [-Serial RFCNC0PWD4E]

param(
    [string]$Serial = "",
    [string]$TermuxPath = "/data/data/com.termux/files/home/deepseek-project"
)

$ErrorActionPreference = "Stop"
$ProjectRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
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

Write-Host "=== DeepSeek Mobile device provision ===" -ForegroundColor Cyan
$devices = Invoke-Adb @("devices")
Write-Host $devices

$pkg = "com.deepseek.mobile"
$hasApp = Invoke-Adb @("shell", "pm", "path", $pkg) 2>$null
if (-not $hasApp) {
    Write-Error "App $pkg not installed. Run: dx build --android --package deepseek-mobile --device <serial>"
}

Invoke-Adb @("shell", "am", "force-stop", $pkg) | Out-Null

Write-Host ""
Write-Host "[1/4] Ensure sandbox workspace directory..."
Invoke-Adb @("shell", "run-as", $pkg, "mkdir", "-p", "files/deepseek-mobile/workspace") | Out-Null
Invoke-Adb @("shell", "run-as", $pkg, "ls", "-la", "files/deepseek-mobile/")

Write-Host ""
Write-Host "[2/4] Check API config (secrets must be created in app UI)..."
$config = Invoke-Adb @("shell", "run-as", $pkg, "cat", "files/deepseek-mobile/config.json") 2>$null
$secrets = Invoke-Adb @("shell", "run-as", $pkg, "ls", "files/deepseek-mobile/secrets.enc") 2>$null
if ($secrets -match "secrets.enc") {
    Write-Host "  secrets.enc present - API key saved on device." -ForegroundColor Green
} else {
    Write-Host "  No secrets.enc - complete first login on the phone (API key)." -ForegroundColor Yellow
}
if ($config) { Write-Host "  config.json present." }

Write-Host ""
Write-Host "[3/4] Termux..."
$termux = Invoke-Adb @("shell", "pm", "list", "packages", "com.termux") 2>$null
if ($termux -match "com.termux") {
    Write-Host "  Termux installed." -ForegroundColor Green
    Write-Host "  On phone: allow-external-apps in ~/.termux/termux.properties, grant RUN_COMMAND."
    Write-Host "  Termux path in app: $TermuxPath"
} else {
    Write-Host "  Termux NOT installed - opening F-Droid on device..." -ForegroundColor Yellow
    Invoke-Adb @("shell", "am", "start", "-a", "android.intent.action.VIEW", "-d", "https://f-droid.org/packages/com.termux/") | Out-Null
    Write-Host "  Install Termux, then re-run this script."
}

Write-Host ""
Write-Host "[4/4] Launch app..."
Invoke-Adb @("shell", "am", "start", "-n", "$pkg/dev.dioxus.main.MainActivity") | Out-Null

Write-Host ""
Write-Host "Done. Save Termux path in app Settings if needed."
Write-Host "Next: Health - Test Termux (pwd) when Termux is ready."
