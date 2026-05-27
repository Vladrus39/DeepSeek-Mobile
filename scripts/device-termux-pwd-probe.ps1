# E2E: agent turn with exec_shell pwd in Termux (YOLO auto-approve, isolated probe thread).
# Does NOT rebuild APK unless -Build. Does NOT force-stop unless -RestartApp.
# Usage: . .\tools\android\env.ps1; .\scripts\device-termux-pwd-probe.ps1 [-Serial RFCNC0PWD4E]

param(
    [string]$Serial = "",
    [switch]$Build,
    [switch]$RestartApp
)

$ErrorActionPreference = "Continue"
$ProjectRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$adb = Join-Path $ProjectRoot "tools\android\sdk\platform-tools\adb.exe"
if (-not (Test-Path $adb)) { $adb = "adb" }

function Invoke-Adb {
    param([string[]]$AdbArgs)
    if ($Serial) { return & $adb -s $Serial @AdbArgs 2>&1 }
    return & $adb @AdbArgs 2>&1
}

$pkg = "com.deepseek.mobile"
$data = "files/deepseek-mobile"
Push-Location $ProjectRoot
try {
    if ($Build) {
        Write-Host "Building APK..." -ForegroundColor Cyan
        if ($Serial) { dx build --android --package deepseek-mobile --device $Serial 2>&1 | Out-Host }
        else { dx build --android --package deepseek-mobile 2>&1 | Out-Host }
        $apk = Join-Path $ProjectRoot "target\dx\deepseek-mobile\debug\android\app\app\build\outputs\apk\debug\app-debug.apk"
        if (Test-Path $apk) { Invoke-Adb @("install", "-r", $apk) | Out-Host }
    }
} finally { Pop-Location }

Invoke-Adb @("shell", "run-as", $pkg, "rm", "-f", "$data/.agent_turn_probe_result") | Out-Null
Invoke-Adb @("shell", "run-as", $pkg, "touch", "$data/.agent_turn_probe_termux_pwd") | Out-Null
Invoke-Adb @("shell", "run-as", $pkg, "touch", "$data/.agent_turn_probe_yolo") | Out-Null
Invoke-Adb @("shell", "run-as", $pkg, "touch", "$data/.agent_turn_probe_requested") | Out-Null

if ($RestartApp) {
    Invoke-Adb @("shell", "am", "force-stop", $pkg) | Out-Null
    Start-Sleep -Seconds 1
}
Invoke-Adb @("shell", "am", "start", "-n", "$pkg/dev.dioxus.main.MainActivity") | Out-Null

Write-Host "Termux pwd probe running - keep app in foreground 90-120s (YOLO, isolated thread)..." -ForegroundColor Yellow
$deadline = (Get-Date).AddSeconds(120)
$result = $null
while ((Get-Date) -lt $deadline) {
    Start-Sleep -Seconds 4
    $raw = Invoke-Adb @("shell", "run-as", $pkg, "cat", "$data/.agent_turn_probe_result")
    $result = ($raw | Out-String).Trim()
    if ($result -and $result -notmatch "No such file") { break }
}

Write-Host "`n=== Termux pwd probe ===" -ForegroundColor Cyan
if ($result) {
    $color = if ($result -match "^PASS") { "Green" } elseif ($result -match "PARTIAL") { "Yellow" } else { "Red" }
    Write-Host $result -ForegroundColor $color
} else {
    Write-Host "FAIL (no result file - app hung or not foreground?)" -ForegroundColor Red
}
