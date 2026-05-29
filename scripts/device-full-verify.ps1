# Full E2E: API + agent turn + Termux cal + PC discovery (optional PC host on LAN).
# Usage: . .\tools\android\env.ps1; .\scripts\device-full-verify.ps1 [-Serial RFCNC0PWD4E] [-SkipBuild] [-SkipPcHost]

param(
    [string]$Serial = "",
    [switch]$SkipBuild,
    [switch]$SkipPcHost
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

function Read-Probe {
    param([string]$Path)
    $out = Invoke-Adb @("shell", "run-as", "com.deepseek.mobile", "cat", $Path) 2>&1
    if ($out -match "No such file") { return $null }
    return ($out | Out-String).Trim()
}

function Wait-Probe {
    param([string]$ResultPath, [int]$Seconds = 60)
    $deadline = (Get-Date).AddSeconds($Seconds)
    while ((Get-Date) -lt $deadline) {
        $r = Read-Probe $ResultPath
        if ($r) { return $r }
        Start-Sleep -Seconds 3
    }
    return $null
}

$pkg = "com.deepseek.mobile"
$results = [ordered]@{}

Write-Host "=== DeepSeek Mobile FULL verify ===" -ForegroundColor Cyan

$pcJob = $null
if (-not $SkipPcHost) {
    Write-Host "`n[PC] deepseek-pc-host on 0.0.0.0:8787 ..." -ForegroundColor Yellow
    $pcAlreadyUp = $false
    try {
        $h = Invoke-RestMethod -Uri "http://127.0.0.1:8787/health" -TimeoutSec 3
        $pcAlreadyUp = $true
        $results["pc_host"] = "PASS (already running: $($h.status))"
    } catch {
        $pcAlreadyUp = $false
    }
    if (-not $pcAlreadyUp) {
        try {
            Push-Location $ProjectRoot
            $pcJob = Start-Job -ScriptBlock {
                param($root)
                Set-Location $root
                $env:DEEPSEEK_PC_HOST_BIND = "0.0.0.0:8787"
                cargo run -q -p deepseek-pc-host 2>&1
            } -ArgumentList $ProjectRoot
            $deadline = (Get-Date).AddSeconds(45)
            do {
                Start-Sleep -Seconds 2
                try {
                    $h = Invoke-RestMethod -Uri "http://127.0.0.1:8787/health" -TimeoutSec 5
                    $results["pc_host"] = "PASS ($($h.status))"
                    break
                } catch {
                    if ((Get-Date) -ge $deadline) {
                        $results["pc_host"] = "FAIL (health timeout: $($_.Exception.Message))"
                    }
                }
            } while ((Get-Date) -lt $deadline -and -not $results["pc_host"])
        } finally {
            Pop-Location
        }
    }
} else {
    $results["pc_host"] = "SKIP"
}

$dev = Invoke-Adb @("devices")
if ($dev -notmatch "device") {
    Write-Host "No device" -ForegroundColor Red
    exit 1
}

if (-not $SkipBuild) {
    Write-Host "`n[Phone] Build + install..." -ForegroundColor Yellow
    Push-Location $ProjectRoot
    if ($Serial) { dx build --android --package deepseek-mobile --device $Serial 2>&1 | Out-Host }
    else { dx build --android --package deepseek-mobile 2>&1 | Out-Host }
    $apk = Join-Path $ProjectRoot "target\dx\deepseek-mobile\debug\android\app\app\build\outputs\apk\debug\app-debug.apk"
    if (Test-Path $apk) { Invoke-Adb @("install", "-r", $apk) | Out-Host }
    Pop-Location
}

Invoke-Adb @("shell", "pm", "grant", $pkg, "com.termux.permission.RUN_COMMAND") | Out-Null

function Start-App {
    Invoke-Adb @("shell", "am", "force-stop", $pkg) | Out-Null
    Start-Sleep -Seconds 1
    Invoke-Adb @("shell", "am", "start", "-n", "$pkg/dev.dioxus.main.MainActivity") | Out-Null
}

Write-Host "`n[Phone] API probe..." -ForegroundColor Yellow
Invoke-Adb @("shell", "run-as", $pkg, "rm", "-f", "files/deepseek-mobile/.api_probe_result") | Out-Null
Invoke-Adb @("shell", "run-as", $pkg, "touch", "files/deepseek-mobile/.api_probe_requested") | Out-Null
Start-App
$results["api_probe"] = Wait-Probe "files/deepseek-mobile/.api_probe_result" 45

Write-Host "`n[Phone] Agent turn probe..." -ForegroundColor Yellow
Invoke-Adb @("shell", "run-as", $pkg, "rm", "-f", "files/deepseek-mobile/.agent_turn_probe_result") | Out-Null
$msgPath = Join-Path $env:TEMP "deepseek-agent-turn-probe.txt"
Set-Content -Path $msgPath -Value "Reply with exactly: PROBE_OK" -Encoding UTF8
Invoke-Adb @("push", $msgPath, "/data/local/tmp/deepseek-agent-turn-probe.txt") | Out-Null
Invoke-Adb @("shell", "run-as", $pkg, "cp", "/data/local/tmp/deepseek-agent-turn-probe.txt", "files/deepseek-mobile/.agent_turn_probe_message") | Out-Null
Invoke-Adb @("shell", "run-as", $pkg, "touch", "files/deepseek-mobile/.agent_turn_probe_requested") | Out-Null
Start-App
$results["agent_turn"] = Wait-Probe "files/deepseek-mobile/.agent_turn_probe_result" 90

Write-Host "`n[Phone] Termux calibration..." -ForegroundColor Yellow
$cal = Read-Probe "files/deepseek-mobile/.agent_calibrated_v1"
$results["termux_cal"] = if ($cal -match "ok") { "PASS" } else { "FAIL (run Termux allow-external-apps)" }

if (-not $SkipPcHost -or $results["pc_host"] -match "^PASS") {
    Write-Host "`n[Phone] PC mDNS discovery..." -ForegroundColor Yellow
    Invoke-Adb @("shell", "run-as", $pkg, "rm", "-f", "files/deepseek-mobile/.pc_discovery_probe_result", "files/deepseek-mobile/.pc_discovery_probe_running") | Out-Null
    Invoke-Adb @("shell", "run-as", $pkg, "touch", "files/deepseek-mobile/.pc_discovery_probe_requested") | Out-Null
    Start-App
    # App Android poll starts after ~4s; probe timeout is 30s in Rust (+12s NSD on device).
    $results["pc_discovery"] = Wait-Probe "files/deepseek-mobile/.pc_discovery_probe_result" 75
}

if ($pcJob) {
    Stop-Job $pcJob -ErrorAction SilentlyContinue
    Remove-Job $pcJob -Force -ErrorAction SilentlyContinue
}

Write-Host "`n=== Summary ===" -ForegroundColor Cyan
foreach ($k in $results.Keys) {
    $c = if ($results[$k] -match "^PASS") { "Green" } elseif ($results[$k] -match "FAIL") { "Red" } else { "Yellow" }
    Write-Host ("  {0,-16} {1}" -f $k, $results[$k]) -ForegroundColor $c
}

$fail = @($results.Values | Where-Object { $_ -match "FAIL" })
if ($fail.Count -gt 0) { exit 1 }
exit 0