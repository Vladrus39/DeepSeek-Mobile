# Quick checks after installing the debug APK on a connected device.
# Usage: .\scripts\device-smoke.ps1 [-Serial RFCNC0PWD4E]

param(
    [string]$Serial = ""
)

$ErrorActionPreference = "Stop"
$ProjectRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$adb = Join-Path $ProjectRoot "tools\android\sdk\platform-tools\adb.exe"
if (-not (Test-Path $adb)) {
    $fallback = Join-Path $env:LOCALAPPDATA "Android\Sdk\platform-tools\adb.exe"
    if (Test-Path $fallback) { $adb = $fallback }
}
if (-not (Test-Path $adb)) {
    $cmd = Get-Command adb -ErrorAction SilentlyContinue
    if ($cmd) { $adb = $cmd.Source }
}
if (-not (Test-Path $adb)) {
    Write-Host "adb not found. Install platform-tools or run: . .\tools\android\env.ps1"
    exit 1
}

$adbArgs = @("devices")
if ($Serial) { $adbArgs = @("-s", $Serial, "devices") }
& adb @adbArgs

$pkg = "com.deepseek.mobile"
$runAs = @("shell", "run-as", $pkg, "ls", "-la", "files/deepseek-mobile")
if ($Serial) { $runAs = @("-s", $Serial) + $runAs }
Write-Host "`n--- app data dir ---"
& adb @runAs 2>&1

Write-Host "`n--- recent logcat (DeepSeek / Rust) ---"
$logArgs = @("logcat", "-d", "-t", "60")
if ($Serial) { $logArgs = @("-s", $Serial) + $logArgs }
& adb @logArgs 2>&1 | Select-String -Pattern "deepseek|DeepSeek|dioxus|RustStdout|Termux" -CaseSensitive:$false

Write-Host "`nDone. On device: Health -> Test Termux (pwd), or Chat quick action Termux pwd."
