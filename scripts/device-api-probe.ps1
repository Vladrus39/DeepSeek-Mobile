# Probe DeepSeek API from the installed app (same HTTP stack as chat).
# Usage: . .\tools\android\env.ps1; .\scripts\device-api-probe.ps1 [-Serial RFCNC0PWD4E] [-SkipBuild]

param(
    [string]$Serial = "",
    [switch]$SkipBuild
)

$ErrorActionPreference = "Stop"
$ProjectRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$adb = Join-Path $ProjectRoot "tools\android\sdk\platform-tools\adb.exe"
if (-not (Test-Path $adb)) { $adb = "adb" }

function Invoke-Adb {
    param([string[]]$AdbArgs)
    $oldEap = $ErrorActionPreference
    $ErrorActionPreference = "Continue"
    try {
        if ($Serial) { return & $adb -s $Serial @AdbArgs 2>&1 }
        return & $adb @AdbArgs 2>&1
    } finally {
        $ErrorActionPreference = $oldEap
    }
}

$pkg = "com.deepseek.mobile"

Push-Location $ProjectRoot
try {
    if (-not $SkipBuild) {
        Write-Host "Building APK..." -ForegroundColor Cyan
        if ($Serial) {
            dx build --android --package deepseek-mobile --device $Serial 2>&1 | Out-Host
        } else {
            dx build --android --package deepseek-mobile 2>&1 | Out-Host
        }
        $apk = Join-Path $ProjectRoot "target\dx\deepseek-mobile\debug\android\app\app\build\outputs\apk\debug\app-debug.apk"
        if (Test-Path $apk) {
            Invoke-Adb @("install", "-r", $apk) | Out-Host
        }
    }
} finally {
    Pop-Location
}

Invoke-Adb @("shell", "run-as", $pkg, "rm", "-f", "files/deepseek-mobile/.api_probe_result") | Out-Null
Invoke-Adb @("shell", "run-as", $pkg, "touch", "files/deepseek-mobile/.api_probe_requested") | Out-Null
Invoke-Adb @("shell", "am", "force-stop", $pkg) | Out-Null
Start-Sleep -Seconds 1
Invoke-Adb @("shell", "am", "start", "-n", "$pkg/dev.dioxus.main.MainActivity") | Out-Null

Write-Host "Waiting for API probe (keep app in foreground ~15s)..." -ForegroundColor Yellow
$deadline = (Get-Date).AddSeconds(45)
$result = $null
while ((Get-Date) -lt $deadline) {
    Start-Sleep -Seconds 3
    $result = Invoke-Adb @("shell", "run-as", $pkg, "cat", "files/deepseek-mobile/.api_probe_result") 2>&1
    if ($result -and $result -notmatch "No such file") {
        break
    }
}

Write-Host "`n=== API probe ===" -ForegroundColor Cyan
if ($result) {
    $color = if ($result -match "^PASS") { "Green" } else { "Red" }
    Write-Host $result -ForegroundColor $color
} else {
    Write-Host "FAIL (no .api_probe_result; keep app foreground ~15s)" -ForegroundColor Red
}

$turn = Invoke-Adb @("shell", "run-as", $pkg, "ls", "-t", "files/deepseek-mobile/runtime_store/turns") 2>&1 | Select-Object -First 1
if ($turn) {
    $latest = ($turn -split "\s+")[-1]
    Write-Host "`nLatest turn:" -ForegroundColor Gray
    Invoke-Adb @("shell", "run-as", $pkg, "cat", "files/deepseek-mobile/runtime_store/turns/$latest") | Out-Host
}
