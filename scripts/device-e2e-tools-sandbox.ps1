# E2E: workspace_overview + apply_patch + read_file on app sandbox (no LLM).
param(
    [string]$Serial = "RFCNC0PWD4E",
    [switch]$Build
)

$ErrorActionPreference = "Continue"
$ProjectRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$adb = Join-Path $ProjectRoot "tools\android\sdk\platform-tools\adb.exe"
if (-not (Test-Path $adb)) { $adb = "adb" }

function Invoke-Adb([string[]]$AdbArgs) {
    if ($Serial) { return & $adb -s $Serial @AdbArgs 2>&1 }
    return & $adb @AdbArgs 2>&1
}

$pkg = "com.deepseek.mobile"
$data = "files/deepseek-mobile"

Push-Location $ProjectRoot
try {
    if ($Build) {
        . .\tools\android\env.ps1
        dx build --android --package deepseek-mobile --device $Serial 2>&1 | Out-Host
        $apk = Join-Path $ProjectRoot "target\dx\deepseek-mobile\debug\android\app\app\build\outputs\apk\debug\app-debug.apk"
        if (Test-Path $apk) { Invoke-Adb @("install", "-r", $apk) | Out-Null }
    }
} finally { Pop-Location }

Write-Host "=== tools sandbox smoke (workspace_overview, apply_patch, read_file) ===" -ForegroundColor Cyan
Invoke-Adb @("shell", "am", "start", "-n", "$pkg/dev.dioxus.main.MainActivity") | Out-Null
Start-Sleep 3
Invoke-Adb @("shell", "run-as", $pkg, "rm", "-f", "$data/.tools_smoke_probe_result") | Out-Null
Invoke-Adb @("shell", "run-as", $pkg, "touch", "$data/.tools_smoke_probe_requested") | Out-Null

$deadline = (Get-Date).AddSeconds(60)
$result = $null
while ((Get-Date) -lt $deadline) {
    Start-Sleep 2
    $raw = Invoke-Adb @("shell", "run-as", $pkg, "cat", "$data/.tools_smoke_probe_result")
    $txt = ($raw | Out-String).Trim()
    if ($txt -and $txt -notmatch "No such file") {
        $result = $txt
        break
    }
}

if (-not $result) {
    Write-Host "FAIL timeout" -ForegroundColor Red
    exit 1
}
Write-Host $result
if ($result -match "^PASS") { exit 0 }
exit 1
