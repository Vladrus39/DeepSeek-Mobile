# End-to-end verification on a connected Android device + local PC Host smoke.
# Usage: . .\tools\android\env.ps1; .\scripts\device-e2e-verify.ps1 [-Serial RFCNC0PWD4E]

param(
    [string]$Serial = "",
    [int]$ForegroundSeconds = 90
)

$ErrorActionPreference = "Stop"
$ProjectRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$adb = Join-Path $ProjectRoot "tools\android\sdk\platform-tools\adb.exe"
if (-not (Test-Path $adb)) { $adb = "adb" }

function Invoke-Adb {
    param([string[]]$AdbArgs)
    if ($Serial) { return & $adb -s $Serial @AdbArgs }
    return & $adb @AdbArgs
}

$pkg = "com.deepseek.mobile"
$results = [ordered]@{}

Write-Host "=== DeepSeek Mobile E2E verify ===" -ForegroundColor Cyan

# --- PC Host (this machine) ---
Write-Host "`n[PC] PC Host health..." -ForegroundColor Yellow
$pcBind = "127.0.0.1:8787"
$pcJob = $null
try {
    Push-Location $ProjectRoot
    $pcJob = Start-Job -ScriptBlock {
        param($root, $bind)
        Set-Location $root
        $env:DEEPSEEK_PC_HOST_BIND = $bind
        cargo run -q -p deepseek-pc-host 2>&1
    } -ArgumentList $ProjectRoot, $pcBind
    Start-Sleep -Seconds 10
    try {
        $health = Invoke-RestMethod -Uri "http://$pcBind/health" -TimeoutSec 5
        $results["pc_host_local"] = "PASS ($($health.status))"
    } catch {
        $results["pc_host_local"] = "FAIL ($($_.Exception.Message))"
    }
} finally {
    if ($pcJob) {
        Stop-Job $pcJob -ErrorAction SilentlyContinue
        Remove-Job $pcJob -Force -ErrorAction SilentlyContinue
    }
    Pop-Location
}

$lanIp = (Get-NetIPAddress -AddressFamily IPv4 -ErrorAction SilentlyContinue |
    Where-Object { $_.IPAddress -notlike "127.*" -and $_.PrefixOrigin -ne "WellKnown" } |
    Select-Object -First 1 -ExpandProperty IPAddress)
if ($lanIp) {
    Write-Host "[PC] LAN IP: $lanIp (phone must be on same Wi‑Fi for mDNS discovery)" -ForegroundColor Gray
    $results["pc_lan_hint"] = "Set DEEPSEEK_PC_HOST_BIND=0.0.0.0:8787 on PC; phone Wi‑Fi: $lanIp"
}

# --- Device ---
$dev = Invoke-Adb @("devices")
if ($dev -notmatch "device`$") {
    Write-Host "No device — skipping phone checks" -ForegroundColor Red
    $results["device"] = "FAIL (not connected)"
} else {
    $results["device"] = "PASS"
    Write-Host "`n[Phone] Build + install..." -ForegroundColor Yellow
    Push-Location $ProjectRoot
    if ($Serial) {
        dx build --android --package deepseek-mobile --device $Serial 2>&1 | Out-Host
    } else {
        dx build --android --package deepseek-mobile 2>&1 | Out-Host
    }
    $apk = Join-Path $ProjectRoot "target\dx\deepseek-mobile\debug\android\app\app\build\outputs\apk\debug\app-debug.apk"
    if (Test-Path $apk) {
        Invoke-Adb @("install", "-r", $apk) | Out-Host
        $results["apk_install"] = "PASS"
    } else {
        $results["apk_install"] = "FAIL (apk missing)"
    }

    Invoke-Adb @("shell", "pm", "grant", $pkg, "com.termux.permission.RUN_COMMAND") | Out-Null
    $grant = Invoke-Adb @("shell", "dumpsys", "package", $pkg) | Select-String "RUN_COMMAND: granted=true"
    $results["termux_permission"] = if ($grant) { "PASS" } else { "FAIL" }

    $termuxPkg = Invoke-Adb @("shell", "pm", "list", "packages", "com.termux")
    $results["termux_installed"] = if ($termuxPkg -match "com.termux") { "PASS" } else { "FAIL" }

    Invoke-Adb @("shell", "run-as", $pkg, "mkdir", "-p", "files/deepseek-mobile/workspace") | Out-Null
    Invoke-Adb @("shell", "run-as", $pkg, "rm", "-f", "files/deepseek-mobile/.agent_calibrated_v1") | Out-Null
    Invoke-Adb @("shell", "run-as", $pkg, "touch", "files/deepseek-mobile/.agent_calibration_requested_v1") | Out-Null
    Invoke-Adb @("shell", "am", "force-stop", $pkg) | Out-Null
    Start-Sleep -Seconds 1
    Invoke-Adb @("shell", "am", "start", "-n", "$pkg/dev.dioxus.main.MainActivity") | Out-Null
    Write-Host "[Phone] Foreground $ForegroundSeconds s for calibration..." -ForegroundColor Yellow
    Start-Sleep -Seconds $ForegroundSeconds

    $calibrated = Invoke-Adb @("shell", "run-as", $pkg, "cat", "files/deepseek-mobile/.agent_calibrated_v1") 2>&1
    $results["termux_calibration"] = if ($calibrated -match "ok") { "PASS" } else { "FAIL (no .agent_calibrated_v1)" }

    $trace = Invoke-Adb @("shell", "run-as", $pkg, "cat", "files/deepseek-mobile/.calibration_trace") 2>&1
    if ($trace -match "stage=marked_ok") { $results["calibration_trace"] = "PASS (marked_ok)" }
    elseif ($trace -match "stage=result") { $results["calibration_trace"] = "PARTIAL (result, not ok)" }
    elseif ($trace -match "termux_failed|stage=queued") { $results["calibration_trace"] = "FAIL ($trace)" }
    else { $results["calibration_trace"] = "UNKNOWN" }

    $log = Invoke-Adb @("logcat", "-d", "-t", "80") | Select-String -Pattern "DeepSeekTermux|termux_failed|RUN_COMMAND" -CaseSensitive:$false
    if ($log) { $results["logcat_termux"] = ($log | Select-Object -Last 3 | Out-String).Trim() }

    Pop-Location
}

Write-Host "`n=== Summary ===" -ForegroundColor Cyan
foreach ($k in $results.Keys) {
    $color = if ($results[$k] -match "^PASS") { "Green" } elseif ($results[$k] -match "FAIL") { "Red" } else { "Yellow" }
    Write-Host ("  {0,-22} {1}" -f $k, $results[$k]) -ForegroundColor $color
}

if ($results["termux_calibration"] -notmatch "^PASS") {
    Write-Host "`nTermux fix (once on phone):" -ForegroundColor Yellow
    Write-Host "  1. Open Termux, finish bootstrap"
    Write-Host "  2. echo allow-external-apps=true >> ~/.termux/termux.properties"
    Write-Host "  3. termux-reload-settings"
    Write-Host "  4. Re-run this script"
}
