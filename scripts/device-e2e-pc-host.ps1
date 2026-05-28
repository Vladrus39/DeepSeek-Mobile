# E2E: PC Host health + phone mDNS discovery (same Wi-Fi).
# Usage: . .\tools\android\env.ps1; .\scripts\device-e2e-pc-host.ps1 [-Serial RFCNC0PWD4E]

param(
    [string]$Serial = "RFCNC0PWD4E",
    [string]$Bind = "0.0.0.0:8787",
    [int]$DiscoveryTimeoutSec = 90
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
$pcJob = $null

function Get-PhoneIpv4 {
    $raw = Invoke-Adb @("shell", "ip", "-4", "addr", "show", "wlan0") 2>&1 | Out-String
    if ($raw -match "inet\s+([0-9.]+)/") { return $Matches[1] }
    return $null
}

function Same-Subnet24([string]$a, [string]$b) {
    if (-not $a -or -not $b) { return $false }
    $pa = $a.Split(".")
    $pb = $b.Split(".")
    if ($pa.Count -lt 3 -or $pb.Count -lt 3) { return $false }
    return ($pa[0] -eq $pb[0]) -and ($pa[1] -eq $pb[1]) -and ($pa[2] -eq $pb[2])
}

Write-Host "=== PC Host E2E ===" -ForegroundColor Cyan

$dev = Invoke-Adb @("devices")
if ($dev -notmatch "`tdevice") {
    Write-Host "FAIL no adb device" -ForegroundColor Red
    exit 1
}

$phoneIp = Get-PhoneIpv4
$pcIps = @(Get-NetIPAddress -AddressFamily IPv4 -ErrorAction SilentlyContinue |
    Where-Object { $_.IPAddress -notmatch '^127\.' -and $_.IPAddress -notmatch '^169\.254\.' } |
    ForEach-Object { $_.IPAddress })
if ($phoneIp) { Write-Host "Phone Wi-Fi: $phoneIp" -ForegroundColor Cyan }
if ($pcIps.Count -gt 0) { Write-Host "PC IPv4: $($pcIps -join ', ')" -ForegroundColor Cyan }
$subnetOk = $false
foreach ($pc in $pcIps) {
    if (Same-Subnet24 $phoneIp $pc) { $subnetOk = $true; break }
}
if ($phoneIp -and $pcIps.Count -gt 0 -and -not $subnetOk) {
    Write-Host "FAIL phone and PC are on different subnets (mDNS/LAN will not work)." -ForegroundColor Red
    Write-Host "Connect both to the SAME Wi-Fi, then re-run. Phone=$phoneIp PC=$($pcIps -join ',')" -ForegroundColor Yellow
    exit 2
}

# 1) Start PC host (or reuse)
try {
    $h = Invoke-RestMethod -Uri "http://127.0.0.1:8787/health" -TimeoutSec 3
    Write-Host "PC host already up: $($h.status) gateway=$($h.gateway_id)" -ForegroundColor Green
} catch {
    Write-Host "Starting deepseek-pc-host on $Bind ..." -ForegroundColor Yellow
    $pcJob = Start-Job -ScriptBlock {
        param($root, $bind)
        Set-Location $root
        $env:DEEPSEEK_PC_HOST_BIND = $bind
        $env:DEEPSEEK_PC_HOST_WORKSPACE = $root
        $env:DEEPSEEK_PC_HOST_LABEL = "DeepSeek-Mobile-PC"
        cargo run -q -p deepseek-pc-host 2>&1
    } -ArgumentList $ProjectRoot, $Bind
    $deadline = (Get-Date).AddSeconds(60)
    $up = $false
    while ((Get-Date) -lt $deadline) {
        Start-Sleep 2
        try {
            $h = Invoke-RestMethod -Uri "http://127.0.0.1:8787/health" -TimeoutSec 5
            Write-Host "PC host health: $($h.status)" -ForegroundColor Green
            $up = $true
            break
        } catch { }
    }
    if (-not $up) {
        Write-Host "FAIL PC host health timeout" -ForegroundColor Red
        if ($pcJob) { Stop-Job $pcJob; Remove-Job $pcJob -Force }
        exit 1
    }
}

# LAN IP for phone reachability hint
$lanIp = $null
$line = (ipconfig | Select-String -Pattern "IPv4" | Select-Object -First 1).Line
if ($line -match ":\s*([0-9.]+)") { $lanIp = $Matches[1] }
if ($lanIp) {
    Write-Host "PC LAN IP (phone must reach): http://${lanIp}:8787/health" -ForegroundColor Cyan
}

# 2) Phone discovery probe
Invoke-Adb @("shell", "run-as", $pkg, "rm", "-f", "$data/.pc_discovery_probe_result", "$data/.pc_discovery_probe_running") | Out-Null
Invoke-Adb @("shell", "run-as", $pkg, "touch", "$data/.pc_discovery_probe_requested") | Out-Null
Invoke-Adb @("shell", "am", "start", "-n", "$pkg/dev.dioxus.main.MainActivity") | Out-Null
Start-Sleep 4

$deadline = (Get-Date).AddSeconds($DiscoveryTimeoutSec)
$discovery = $null
while ((Get-Date) -lt $deadline) {
    Start-Sleep 3
    $raw = Invoke-Adb @("shell", "run-as", $pkg, "cat", "$data/.pc_discovery_probe_result")
    $txt = ($raw | Out-String).Trim()
    if ($txt -and $txt -notmatch "No such file") {
        $discovery = $txt
        break
    }
}

if ($pcJob) {
    Stop-Job $pcJob -ErrorAction SilentlyContinue
    Remove-Job $pcJob -Force -ErrorAction SilentlyContinue
}

if (-not $discovery) {
    Write-Host "FAIL phone discovery timeout" -ForegroundColor Red
    exit 1
}

Write-Host "Discovery: $discovery" -ForegroundColor $(if ($discovery -match "^PASS") { "Green" } else { "Red" })

if ($discovery -notmatch "^PASS") {
    exit 1
}

# 3) Optional: hit discovered URL from PC (same machine as host)
if ($discovery -match "first=(http://[^;\s]+)") {
    $url = $Matches[1].TrimEnd('/') + "/health"
    try {
        $rh = Invoke-RestMethod -Uri $url -TimeoutSec 5
        Write-Host "Health via discovered URL: $($rh.status)" -ForegroundColor Green
    } catch {
        Write-Host "WARN health via discovered URL failed (firewall?): $_" -ForegroundColor Yellow
    }
}

Write-Host "PASS PC Host E2E" -ForegroundColor Green
exit 0
