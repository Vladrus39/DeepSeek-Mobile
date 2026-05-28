# E2E: trigger ZIP export+share probe on device (app must be foreground).
param(
    [string]$Device = "RFCNC0PWD4E",
    [string]$Package = "com.deepseek.mobile",
    [int]$TimeoutSec = 45
)

$ErrorActionPreference = "Continue"
$root = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$adb = Join-Path $root "tools\android\sdk\platform-tools\adb.exe"
$AdbArgs = @("-s", $Device)

function Invoke-Adb([string[]]$Cmd) {
    & $adb @AdbArgs @Cmd 2>&1
}

Write-Host "Starting app..." -ForegroundColor Cyan
Invoke-Adb @("shell", "am", "start", "-n", "$Package/dev.dioxus.main.MainActivity") | Out-Null
Start-Sleep 3

$base = "files/deepseek-mobile"
Invoke-Adb @("shell", "run-as", $Package, "rm", "-f", "$base/.zip_transfer_probe_result")
Invoke-Adb @("shell", "run-as", $Package, "touch", "$base/.zip_transfer_probe_requested")

$deadline = (Get-Date).AddSeconds($TimeoutSec)
$result = $null
while ((Get-Date) -lt $deadline) {
    Start-Sleep 2
    $raw = Invoke-Adb @("shell", "run-as", $Package, "cat", "$base/.zip_transfer_probe_result")
    $txt = ($raw | Out-String).Trim()
    if ($txt -and $txt -notmatch "No such file") {
        $result = $txt
        break
    }
}

if (-not $result) {
    Write-Host "FAIL timeout waiting for probe result" -ForegroundColor Red
    Invoke-Adb @("logcat", "-d", "-t", "200") | Select-String -Pattern "share|FileProvider|deepseek" | Select-Object -Last 30
    exit 1
}

Write-Host $result
if ($result -match "^PASS") { exit 0 }
exit 1
