# Capture a REAL chat-error logcat from the connected phone.
#
# Usage (PowerShell, from the project root):
#   .\scripts\capture-chat-log.ps1
#
# It clears logcat, then records everything while you reproduce the error
# in the app, and saves it to target\chat-error-capture.txt.
# That file lives in the project folder, so Claude can read it directly.

$ErrorActionPreference = "Stop"

$root  = Split-Path -Parent $PSScriptRoot
$adb   = Join-Path $root "tools\android\sdk\platform-tools\adb.exe"
$pkg   = "com.deepseek.mobile"
$out   = Join-Path $root "target\chat-error-capture.txt"

if (-not (Test-Path $adb)) {
    Write-Error "adb not found at $adb. Run .\tools\android\env.ps1 / sync-sdk-from-system.ps1 first."
    exit 1
}

Write-Host "== Connected devices ==" -ForegroundColor Cyan
& $adb devices

# Resolve the app PID (launch it if needed)
$appPid = (& $adb shell pidof $pkg 2>$null | Out-String).Trim()
if (-not $appPid) {
    Write-Host "App not running - launching $pkg ..." -ForegroundColor Yellow
    & $adb shell monkey -p $pkg -c android.intent.category.LAUNCHER 1 | Out-Null
    Start-Sleep -Seconds 3
    $appPid = (& $adb shell pidof $pkg 2>$null | Out-String).Trim()
}
Write-Host "App PID: $appPid" -ForegroundColor Green

# Clear old logs
& $adb logcat -c

Write-Host ""
Write-Host "=== RECORDING. Now do this on the phone: ===" -ForegroundColor Magenta
Write-Host "  1. Open the chat" -ForegroundColor Magenta
Write-Host "  2. Send a message (e.g. 'create a small python project')" -ForegroundColor Magenta
Write-Host "  3. Wait until the error appears on screen" -ForegroundColor Magenta
Write-Host ""

# Record full logcat to file in the background
$job = Start-Job -ScriptBlock {
    param($adb, $out)
    & $adb logcat -v time *:V | Out-File -Encoding utf8 -FilePath $out
} -ArgumentList $adb, $out

Read-Host "When the error is visible on the phone, press ENTER to stop recording"

Stop-Job  $job  -ErrorAction SilentlyContinue
Remove-Job $job -Force -ErrorAction SilentlyContinue

$size = (Get-Item $out -ErrorAction SilentlyContinue).Length
Write-Host ""
Write-Host "Saved capture to: $out  ($size bytes)" -ForegroundColor Green
Write-Host "Now tell Claude: 'I captured the log' and it will read it." -ForegroundColor Cyan
