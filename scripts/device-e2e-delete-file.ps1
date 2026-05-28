# E2E: delete_file in Termux workspace via agent_turn_probe (YOLO).
# Usage: . .\tools\android\env.ps1; .\scripts\device-e2e-delete-file.ps1 -Serial RFCNC0PWD4E
# Keep app in foreground ~2 min per step.

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
$target = "test_e2e_delete_me.txt"

Push-Location $ProjectRoot
try {
    if ($Build) {
        dx build --android --package deepseek-mobile --device $Serial 2>&1 | Out-Host
        $apk = Join-Path $ProjectRoot "target\dx\deepseek-mobile\debug\android\app\app\build\outputs\apk\debug\app-debug.apk"
        if (Test-Path $apk) { Invoke-Adb @("install", "-r", $apk) | Out-Host }
    }
} finally { Pop-Location }

function Set-ProbeMessage([string]$Text) {
    $tmp = Join-Path $env:TEMP "deepseek-delete-probe.txt"
    Set-Content -Path $tmp -Value $Text -Encoding UTF8
    Invoke-Adb @("push", $tmp, "/data/local/tmp/deepseek-delete-probe.txt") | Out-Null
    Invoke-Adb @("shell", "run-as", $pkg, "cp", "/data/local/tmp/deepseek-delete-probe.txt", "$data/.agent_turn_probe_message") | Out-Null
}

function Start-YoloProbe {
    Invoke-Adb @("shell", "run-as", $pkg, "rm", "-f", "$data/.agent_turn_probe_result", "$data/.agent_turn_probe_termux_pwd") | Out-Null
    Invoke-Adb @("shell", "run-as", $pkg, "touch", "$data/.agent_turn_probe_yolo") | Out-Null
    Invoke-Adb @("shell", "run-as", $pkg, "touch", "$data/.agent_turn_probe_requested") | Out-Null
    Invoke-Adb @("shell", "am", "start", "-n", "$pkg/dev.dioxus.main.MainActivity") | Out-Null
}

function Wait-Probe([int]$Sec = 150) {
    $deadline = (Get-Date).AddSeconds($Sec)
    while ((Get-Date) -lt $deadline) {
        Start-Sleep -Seconds 4
        $raw = Invoke-Adb @("shell", "run-as", $pkg, "cat", "$data/.agent_turn_probe_result")
        $r = ($raw | Out-String).Trim()
        if ($r -and $r -notmatch "No such file") { return $r }
    }
    return $null
}

Write-Host "=== delete_file E2E (Termux, app foreground) ===" -ForegroundColor Cyan

$createMsg = @"
In Termux workspace use write_file only. First response JSON only: {"tool":"write_file","args":{"path":"$target","content":"DELETE_ME"}}. Reply CREATE_OK only.
"@
Set-ProbeMessage $createMsg
Start-YoloProbe
Write-Host "Step 1: create $target ..." -ForegroundColor Yellow
$create = Wait-Probe
if (-not $create -or $create -notmatch "^PASS termux") {
    Write-Host "FAIL create: $create" -ForegroundColor Red
    exit 1
}
Write-Host $create -ForegroundColor Green

$deleteMsg = @"
In Termux workspace use delete_file only. First response JSON only: {"tool":"delete_file","args":{"path":"$target"}}. Reply DELETE_OK only.
"@
Set-ProbeMessage $deleteMsg
Start-YoloProbe
Write-Host "Step 2: delete_file $target ..." -ForegroundColor Yellow
$delete = Wait-Probe
if (-not $delete -or $delete -notmatch "^PASS termux") {
    Write-Host "FAIL delete: $delete" -ForegroundColor Red
    exit 1
}
Write-Host $delete -ForegroundColor Green

$verifyMsg = @"
Use exec_shell only. First JSON: {"tool":"exec_shell","args":{"command":"test ! -f $target && echo /data/GONE || echo STILL_THERE","timeout_secs":30}}. Reply with command stdout only.
"@
Set-ProbeMessage $verifyMsg
Start-YoloProbe
Write-Host "Step 3: verify file absent ..." -ForegroundColor Yellow
$verify = Wait-Probe
if (-not $verify -or $verify -notmatch "^PASS termux" -or $verify -notmatch 'stdout=.*GONE') {
    Write-Host "FAIL verify: $verify" -ForegroundColor Red
    exit 1
}
Write-Host $verify -ForegroundColor Green
Write-Host "PASS delete_file E2E" -ForegroundColor Green
exit 0
