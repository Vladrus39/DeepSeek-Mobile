# Run automated device E2E probes and refresh docs/DEVICE_CHECKLIST.md summary.
# Usage: .\scripts\run-device-checklist.ps1 [-Serial RFCNC0PWD4E] [-WaitForDeviceSeconds 120]

param(
    [string]$Serial = "",
    [int]$WaitForDeviceSeconds = 120,
    [switch]$SkipBuild,
    [switch]$SkipPcHost
)

$ErrorActionPreference = "Stop"
$ProjectRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$adb = Join-Path $ProjectRoot "tools\android\sdk\platform-tools\adb.exe"
if (-not (Test-Path $adb)) { $adb = "adb" }

function Get-AdbSerial {
    param([string]$Preferred)
    if ($Preferred) {
        $state = & $adb -s $Preferred get-state 2>&1
        if ($LASTEXITCODE -eq 0 -and "$state" -eq "device") { return $Preferred }
    }
    $lines = @(& $adb devices 2>&1 | Where-Object { $_ -match "^\S+\s+device\s*$" })
    if ($lines.Count -ge 1) {
        return ($lines[0] -split "\s+", 2)[0]
    }
    return $null
}

$deadline = (Get-Date).AddSeconds($WaitForDeviceSeconds)
$serial = $null
while ((Get-Date) -lt $deadline) {
    $serial = Get-AdbSerial -Preferred $Serial
    if ($serial) { break }
    Write-Host "Waiting for USB device (adb)..." -ForegroundColor Yellow
    Start-Sleep -Seconds 3
}
if (-not $serial) {
    throw "No adb device in 'device' state. Enable USB debugging and authorize this PC."
}
Write-Host "Using device serial: $serial" -ForegroundColor Green

Set-Location $ProjectRoot
$envScript = Join-Path $ProjectRoot "tools\android\env.ps1"
if (Test-Path $envScript) { . $envScript }

$results = [ordered]@{}
function Run-Step([string]$Name, [scriptblock]$Block) {
    Write-Host "`n=== $Name ===" -ForegroundColor Cyan
    try {
        & $Block
        if ($LASTEXITCODE -ne 0) { $results[$Name] = "FAIL (exit $LASTEXITCODE)" }
        else { $results[$Name] = "PASS" }
    } catch {
        $results[$Name] = "FAIL ($($_.Exception.Message))"
    }
    Write-Host "$Name -> $($results[$Name])" -ForegroundColor $(if ($results[$Name] -eq "PASS") { "Green" } else { "Red" })
}

if (-not $SkipBuild) {
    Run-Step "install_debug_apk" {
        & (Join-Path $ProjectRoot "scripts\update-phone-apk.ps1") -Serial $serial -SkipTests -Launch
    }
}

Run-Step "zip_import" {
    & (Join-Path $ProjectRoot "scripts\device-e2e-zip-import.ps1") -Serial $serial
}
Run-Step "zip_export" {
    & (Join-Path $ProjectRoot "scripts\device-e2e-zip-export.ps1") -Serial $serial
}
Run-Step "termux_pwd" {
    & (Join-Path $ProjectRoot "scripts\device-termux-pwd-probe.ps1") -Serial $serial
}
Run-Step "pc_host" {
    & (Join-Path $ProjectRoot "scripts\device-e2e-pc-host.ps1") -Serial $serial
}
Run-Step "pc_pairing_bundle" {
    & (Join-Path $ProjectRoot "scripts\device-e2e-pc-pairing-bundle.ps1") -Serial $serial -SkipBuild
}
Run-Step "device_full_verify" {
    $fvArgs = @("-Serial", $serial, "-SkipBuild")
    if ($SkipPcHost) { $fvArgs += "-SkipPcHost" }
    & (Join-Path $ProjectRoot "scripts\device-full-verify.ps1") @fvArgs
}

Write-Host "`n=== Summary ===" -ForegroundColor Cyan
$results.GetEnumerator() | ForEach-Object { Write-Host ("  {0,-22} {1}" -f $_.Key, $_.Value) }
$fail = @($results.Values | Where-Object { $_ -notmatch "^PASS" })
if ($fail.Count -gt 0) { exit 1 }
exit 0
