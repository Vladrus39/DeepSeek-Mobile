# E2E: APK exports PC pairing ZIP -> pull -> unzip on PC -> launch bundle -> health + mDNS discovery.
# Usage: . .\tools\android\env.ps1; .\scripts\device-e2e-pc-pairing-bundle.ps1 [-Serial RFCNC0PWD4E] [-SkipBuild]

param(
    [string]$Serial = "RFCNC0PWD4E",
    [string]$Package = "com.deepseek.mobile",
    [switch]$SkipBuild,
    [int]$ExportTimeoutSec = 45,
    [int]$DiscoveryTimeoutSec = 90
)

$ErrorActionPreference = "Stop"
$ProjectRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$adb = Join-Path $ProjectRoot "tools\android\sdk\platform-tools\adb.exe"
if (-not (Test-Path $adb)) { $adb = "adb" }

function Invoke-Adb([string[]]$AdbArgs) {
    $prev = $ErrorActionPreference
    $ErrorActionPreference = "Continue"
    try {
        if ($Serial) { return & $adb -s $Serial @AdbArgs 2>&1 }
        return & $adb @AdbArgs 2>&1
    } finally {
        $ErrorActionPreference = $prev
    }
}

function Test-AdbDevice {
    $dev = (Invoke-Adb @("devices") | Out-String)
    return $dev -match "`tdevice"
}

function Stop-PcHostOnPort([int]$Port = 8787) {
    Get-NetTCPConnection -LocalPort $Port -ErrorAction SilentlyContinue |
        ForEach-Object {
            if ($_.OwningProcess -gt 0) {
                Stop-Process -Id $_.OwningProcess -Force -ErrorAction SilentlyContinue
            }
        }
    Start-Sleep -Seconds 1
}

$base = "files/deepseek-mobile"
$outRoot = Join-Path $ProjectRoot "target\pc-pairing-e2e"
$bundleDir = Join-Path $outRoot "bundle"
$zipLocal = Join-Path $outRoot "pairing-bundle.zip"

Write-Host "=== PC Pairing Bundle E2E (APK -> PC) ===" -ForegroundColor Cyan

if (-not (Test-AdbDevice)) {
    Write-Host "FAIL no adb device" -ForegroundColor Red
    exit 1
}

if (-not $SkipBuild) {
    Write-Host "Building APK..." -ForegroundColor Yellow
    Push-Location $ProjectRoot
    $prevEap = $ErrorActionPreference
    $ErrorActionPreference = "Continue"
    try {
        if ($Serial) {
            & dx build --android --package deepseek-mobile --device $Serial 2>&1 | Out-Host
        } else {
            & dx build --android --package deepseek-mobile 2>&1 | Out-Host
        }
        $apk = Join-Path $ProjectRoot "target\dx\deepseek-mobile\debug\android\app\app\build\outputs\apk\debug\app-debug.apk"
        if (Test-Path $apk) {
            Invoke-Adb @("install", "-r", $apk) | Out-Host
        } else {
            Write-Host "WARN APK not found after build; using installed app" -ForegroundColor Yellow
        }
    } finally {
        $ErrorActionPreference = $prevEap
        Pop-Location
    }
}

Write-Host "Building deepseek-pc-host (release)..." -ForegroundColor Yellow
Push-Location $ProjectRoot
cargo build -q -p deepseek-pc-host --release 2>&1 | Out-Host
Pop-Location
$hostExe = Join-Path $ProjectRoot "target\release\deepseek-pc-host.exe"
if (-not (Test-Path $hostExe)) {
    Write-Host "FAIL missing $hostExe" -ForegroundColor Red
    exit 1
}

Write-Host "Trigger pairing ZIP export on device..." -ForegroundColor Cyan
Invoke-Adb @("shell", "am", "start", "-n", "$Package/dev.dioxus.main.MainActivity") | Out-Null
Start-Sleep 3
Invoke-Adb @("shell", "run-as", $Package, "rm", "-f", "$base/.pc_pairing_bundle_probe_result") | Out-Null
Invoke-Adb @("shell", "run-as", $Package, "touch", "$base/.pc_pairing_bundle_probe_requested") | Out-Null

$exportLine = $null
$deadline = (Get-Date).AddSeconds($ExportTimeoutSec)
while ((Get-Date) -lt $deadline) {
    Start-Sleep 2
    $raw = Invoke-Adb @("shell", "run-as", $Package, "cat", "$base/.pc_pairing_bundle_probe_result")
    $txt = ($raw | Out-String).Trim()
    if ($txt -and $txt -notmatch "No such file") {
        $exportLine = $txt
        break
    }
}
if (-not $exportLine) {
    Write-Host "FAIL timeout waiting for pairing bundle export" -ForegroundColor Red
    exit 2
}
Write-Host "Export: $exportLine" -ForegroundColor Gray
if ($exportLine -notmatch "^PASS zip=(.+) gateway_id=(\S+)") {
    Write-Host "FAIL $exportLine" -ForegroundColor Red
    exit 2
}
$deviceZip = $Matches[1].Trim()
$expectedGatewayId = $Matches[2].Trim()

Write-Host "Pull ZIP from device: $deviceZip" -ForegroundColor Cyan
New-Item -ItemType Directory -Force -Path $outRoot | Out-Null
$zipName = [System.IO.Path]::GetFileName($deviceZip)
$relZip = "$base/pairing-export/$zipName"
$sdcardZip = "/sdcard/Download/deepseek-pairing-bundle-e2e.zip"
$cpOut = Invoke-Adb @("shell", "run-as", $Package, "cp", $relZip, $sdcardZip)
$cpText = ($cpOut | Out-String).Trim()
if ($cpText -match "No such file|Permission denied|not found") {
    Write-Host "WARN sdcard cp failed ($cpText); exec-out via cmd" -ForegroundColor Yellow
    if (Test-Path $zipLocal) { Remove-Item -Force $zipLocal }
    $pullCmd = "`"$adb`" -s $Serial exec-out run-as $Package cat $relZip"
    cmd /c "$pullCmd > `"$zipLocal`""
} else {
    if (Test-Path $zipLocal) { Remove-Item -Force $zipLocal }
    Invoke-Adb @("pull", $sdcardZip, $zipLocal) | Out-Null
    Invoke-Adb @("shell", "rm", "-f", $sdcardZip) | Out-Null
}
if (-not (Test-Path $zipLocal) -or (Get-Item $zipLocal).Length -lt 100) {
    Write-Host "FAIL could not pull pairing zip" -ForegroundColor Red
    exit 3
}

if (Test-Path $bundleDir) { Remove-Item -Recurse -Force $bundleDir }
New-Item -ItemType Directory -Force -Path $bundleDir | Out-Null
Expand-Archive -Path $zipLocal -DestinationPath $bundleDir -Force
Copy-Item -Force $hostExe (Join-Path $bundleDir "deepseek-pc-host.exe")

$pairingJson = Join-Path $bundleDir "pairing.json"
if (-not (Test-Path $pairingJson)) {
    Write-Host "FAIL pairing.json missing in bundle" -ForegroundColor Red
    exit 4
}
$pairing = Get-Content $pairingJson -Raw | ConvertFrom-Json
$envFile = Join-Path $bundleDir "deepseek-pc-host.env"
$envContent = Get-Content $envFile -Raw
$envContent = $envContent -replace '(?m)^DEEPSEEK_PC_HOST_WORKSPACE=.*$', "DEEPSEEK_PC_HOST_WORKSPACE=$ProjectRoot"
Set-Content -Path $envFile -Value $envContent -NoNewline

try {
    $existing = Invoke-RestMethod -Uri "http://127.0.0.1:8787/health" -TimeoutSec 3
    Write-Host "PC host already healthy (gateway_id=$($existing.gateway_id)); skip bundle relaunch" -ForegroundColor Green
    $healthOk = $true
    $skipBundleLaunch = $true
} catch {
    $skipBundleLaunch = $false
    $healthOk = $false
}
if (-not $skipBundleLaunch) {
Write-Host "Stop any existing PC host on :8787..." -ForegroundColor Yellow
Stop-PcHostOnPort 8787

Write-Host "Launch PC host from pairing bundle (start-deepseek-pc-host.ps1)..." -ForegroundColor Cyan
$launcher = Join-Path $bundleDir "start-deepseek-pc-host.ps1"
$pcJob = Start-Job -ScriptBlock {
    param($launcherPath, $bundlePath, $workspace)
    Set-Location $bundlePath
    Get-Content (Join-Path $bundlePath "deepseek-pc-host.env") | ForEach-Object {
        if ($_ -match '^\s*([^#=]+)=(.*)$') {
            Set-Item -Path ("Env:" + $matches[1].Trim()) -Value $matches[2].Trim()
        }
    }
    $env:DEEPSEEK_PC_HOST_WORKSPACE = $workspace
    & $launcherPath 2>&1
} -ArgumentList $launcher, $bundleDir, $ProjectRoot

$healthOk = $false
$healthDeadline = (Get-Date).AddSeconds(90)
while ((Get-Date) -lt $healthDeadline) {
    Start-Sleep 2
    try {
        $h = Invoke-RestMethod -Uri "http://127.0.0.1:8787/health" -TimeoutSec 3
        if ($h.status) {
            Write-Host "PC host up: $($h.status) gateway_id=$($h.gateway_id)" -ForegroundColor Green
            $healthOk = $true
            break
        }
        Write-Host "WARN health gateway_id=$($h.gateway_id) expected $expectedGatewayId" -ForegroundColor Yellow
    } catch {}
}
if (-not $healthOk) {
    Write-Host "FAIL PC host did not become healthy from pairing bundle" -ForegroundColor Red
    Receive-Job $pcJob -ErrorAction SilentlyContinue | Select-Object -Last 20
    Stop-Job $pcJob -ErrorAction SilentlyContinue
    Remove-Job $pcJob -Force -ErrorAction SilentlyContinue
    exit 5
}

}
Write-Host "Phone mDNS discovery..." -ForegroundColor Cyan
& (Join-Path $PSScriptRoot "device-e2e-pc-host.ps1") -Serial $Serial -DiscoveryTimeoutSec $DiscoveryTimeoutSec
$discExit = $LASTEXITCODE

if ($discExit -ne 0) {
    Write-Host "WARN mDNS discovery failed; probing manual LAN URL..." -ForegroundColor Yellow
    $pcIp = (Get-NetIPAddress -AddressFamily IPv4 -ErrorAction SilentlyContinue |
        Where-Object { $_.IPAddress -match '^192\.168\.' } |
        Select-Object -First 1).IPAddress
    if ($pcIp) {
        try {
            $manual = Invoke-RestMethod -Uri "http://${pcIp}:8787/health" -TimeoutSec 5
            Write-Host "Manual LAN health OK: gateway_id=$($manual.gateway_id)" -ForegroundColor Green
            $discExit = 0
        } catch {
            Write-Host "FAIL manual LAN health: $_" -ForegroundColor Red
        }
    }
}

Write-Host ""
Write-Host "Bundle dir: $bundleDir" -ForegroundColor Cyan
Write-Host "Pairing ZIP: $zipLocal" -ForegroundColor Cyan
if ($discExit -eq 0) {
    Write-Host "PASS PC pairing bundle E2E (export + PC launch + reachability)" -ForegroundColor Green
    exit 0
}
Write-Host "PARTIAL PASS export+launch OK; discovery/manual check failed" -ForegroundColor Yellow
exit 6
