# Full phone agent calibration via ADB.
# Usage: . .\tools\android\env.ps1; .\scripts\device-calibrate.ps1 [-Serial RFCNC0PWD4E] [-SkipBuild]

param(
    [string]$Serial = "",
    [switch]$SkipBuild,
    [switch]$InteractiveTermuxSetup
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

function Assert-DeviceAuthorized {
    $devices = Invoke-Adb @("devices")
    Write-Host $devices
    if ($devices -match "unauthorized") {
        throw "ADB device is unauthorized. Unlock the phone and accept the USB debugging/RSA prompt."
    }
    if ($devices -notmatch "device`r?`n|device$") {
        throw "No authorized Android device found."
    }
}

$pkg = "com.deepseek.mobile"
$apk = Join-Path $ProjectRoot "target\dx\deepseek-mobile\debug\android\app\app\build\outputs\apk\debug\app-debug.apk"

Write-Host "=== DeepSeek Mobile full agent calibration ===" -ForegroundColor Cyan
Assert-DeviceAuthorized

Push-Location $ProjectRoot
try {
    if (-not $SkipBuild) {
        Write-Host "Building latest debug APK..." -ForegroundColor Cyan
        if ($Serial) {
            dx build --android --package deepseek-mobile --device $Serial 2>&1 | Out-Host
        } else {
            dx build --android --package deepseek-mobile 2>&1 | Out-Host
        }
    }

    if (-not (Test-Path $apk)) {
        throw "APK missing: $apk"
    }

    Write-Host "Installing APK..." -ForegroundColor Cyan
    Invoke-Adb @("install", "-r", $apk) | Out-Host
} finally {
    Pop-Location
}

$termuxPkg = Invoke-Adb @("shell", "pm", "list", "packages", "com.termux")
if ($termuxPkg -notmatch "com.termux") {
    Write-Host "Termux missing - installing from PC..." -ForegroundColor Yellow
    & (Join-Path $ProjectRoot "scripts\install-termux-offline.ps1") -Serial $Serial
} else {
    Write-Host "Termux: installed" -ForegroundColor Green
}

Write-Host "Granting Termux RUN_COMMAND to DeepSeek..." -ForegroundColor Cyan
Invoke-Adb @("shell", "pm", "grant", $pkg, "com.termux.permission.RUN_COMMAND") 2>$null | Out-Null

Write-Host "Termux one-time setup (required for background RUN_COMMAND):" -ForegroundColor Yellow
Write-Host "  mkdir -p ~/.termux ~/deepseek-project"
Write-Host "  echo allow-external-apps=true >> ~/.termux/termux.properties"
Write-Host "  termux-reload-settings"
Invoke-Adb @("shell", "am", "start", "-n", "com.termux/.app.TermuxActivity") | Out-Null
if ($InteractiveTermuxSetup) {
    Write-Host "  Run the lines above in Termux, then press Enter..." -ForegroundColor Yellow
    Read-Host "Press Enter after Termux setup"
    Invoke-Adb @("shell", "am", "force-stop", "com.termux") | Out-Null
    Start-Sleep -Seconds 2
}

Write-Host "Seeding app sandbox workspace and requesting calibration..." -ForegroundColor Cyan
Invoke-Adb @("shell", "run-as", $pkg, "mkdir", "-p", "files/deepseek-mobile/workspace") | Out-Null
$readme = @"
# DeepSeek Mobile (sandbox)

Lite workspace inside the app. Full shell/git/build runs in Termux.

Termux path: /data/data/com.termux/files/home/deepseek-project
"@
$readmePath = Join-Path $env:TEMP "deepseek-sandbox-readme.md"
Set-Content -Path $readmePath -Value $readme -Encoding UTF8
Invoke-Adb @("push", $readmePath, "/data/local/tmp/deepseek-readme.md") | Out-Null
Invoke-Adb @("shell", "run-as", $pkg, "cp", "/data/local/tmp/deepseek-readme.md", "files/deepseek-mobile/workspace/README.md") | Out-Null
Invoke-Adb @("shell", "run-as", $pkg, "rm", "-f", "files/deepseek-mobile/.agent_calibrated_v1") | Out-Null
Invoke-Adb @("shell", "run-as", $pkg, "touch", "files/deepseek-mobile/.agent_calibration_requested_v1") | Out-Null

Write-Host "Launching app (explicit calibration request is set)..." -ForegroundColor Cyan
Invoke-Adb @("shell", "am", "force-stop", $pkg) | Out-Null
Start-Sleep -Seconds 1
Invoke-Adb @("shell", "am", "start", "-n", "$pkg/dev.dioxus.main.MainActivity") | Out-Null

Write-Host ""
Write-Host "Done. On the phone:" -ForegroundColor Green
Write-Host "  - Keep DeepSeek Mobile open ~30-90s until timeline shows calibration OK."
Write-Host "  - Then Chat: 'выполни pwd и ls в termux' or Health -> Test Termux."
Write-Host ""
Write-Host "If calibration fails: open Termux once, finish bootstrap, run:"
Write-Host "  mkdir -p ~/.termux && echo allow-external-apps=true >> ~/.termux/termux.properties && termux-reload-settings"
