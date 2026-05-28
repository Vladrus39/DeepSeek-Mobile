# E2E: headless ZIP import probe (no system picker).
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

$staging = Join-Path $root "target\zip-import-probe"
New-Item -ItemType Directory -Force -Path $staging | Out-Null
Set-Content -Path (Join-Path $staging "import_probe_marker.txt") -Value "ZIP_IMPORT_E2E" -Encoding utf8
$zipPath = Join-Path $root "target\import-test.zip"
if (Test-Path $zipPath) { Remove-Item $zipPath -Force }
Compress-Archive -Path (Join-Path $staging "*") -DestinationPath $zipPath -Force

Write-Host "Starting app..." -ForegroundColor Cyan
Invoke-Adb @("shell", "am", "start", "-n", "$Package/dev.dioxus.main.MainActivity") | Out-Null
Start-Sleep 3

$base = "files/deepseek-mobile"
Invoke-Adb @("shell", "run-as", $Package, "mkdir", "-p", "$base/probes") | Out-Null
Invoke-Adb @("push", $zipPath, "/data/local/tmp/import-test.zip") | Out-Null
Invoke-Adb @("shell", "run-as", $Package, "cp", "/data/local/tmp/import-test.zip", "$base/probes/import-test.zip") | Out-Null
Invoke-Adb @("shell", "run-as", $Package, "rm", "-f", "$base/.zip_transfer_probe_result") | Out-Null
Invoke-Adb @("shell", "run-as", $Package, "touch", "$base/.zip_transfer_probe_import_requested") | Out-Null

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
    Write-Host "FAIL timeout waiting for import probe" -ForegroundColor Red
    exit 1
}

Write-Host $result
if ($result -match "^PASS") { exit 0 }
exit 1
