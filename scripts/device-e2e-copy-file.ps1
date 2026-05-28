# E2E: copy_file in Termux workspace via agent_turn_probe (YOLO).
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
$src = "test_e2e_copy_src.txt"
$dst = "test_e2e_copy_dst.txt"

Push-Location $ProjectRoot
try {
    if ($Build) {
        dx build --android --package deepseek-mobile --device $Serial 2>&1 | Out-Host
        $apk = Join-Path $ProjectRoot "target\dx\deepseek-mobile\debug\android\app\app\build\outputs\apk\debug\app-debug.apk"
        if (Test-Path $apk) { Invoke-Adb @("install", "-r", $apk) | Out-Null }
    }
} finally { Pop-Location }

function Set-ProbeMessage([string]$Text) {
    $tmp = Join-Path $env:TEMP "deepseek-copy-probe.txt"
    Set-Content -Path $tmp -Value $Text -Encoding UTF8
    Invoke-Adb @("push", $tmp, "/data/local/tmp/deepseek-copy-probe.txt") | Out-Null
    Invoke-Adb @("shell", "run-as", $pkg, "cp", "/data/local/tmp/deepseek-copy-probe.txt", "$data/.agent_turn_probe_message") | Out-Null
}

function Start-YoloProbe {
    Invoke-Adb @("shell", "run-as", $pkg, "rm", "-f", "$data/.agent_turn_probe_result") | Out-Null
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

Write-Host "=== copy_file E2E (Termux) ===" -ForegroundColor Cyan

$createMsg = @"
In Termux workspace use write_file only. First response JSON only: {"tool":"write_file","args":{"path":"$src","content":"COPY_SRC"}}. Reply CREATE_OK only.
"@
Set-ProbeMessage $createMsg
Start-YoloProbe
Write-Host "Step 1: create $src ..." -ForegroundColor Yellow
$c = Wait-Probe
if (-not $c -or $c -notmatch "^PASS termux") {
    Write-Host "FAIL create: $c" -ForegroundColor Red
    exit 1
}

$copyMsg = @"
In Termux workspace use copy_file only. First response JSON only: {"tool":"copy_file","args":{"source":"$src","dest":"$dst"}}. Reply COPY_OK only.
"@
Set-ProbeMessage $copyMsg
Start-YoloProbe
Write-Host "Step 2: copy_file -> $dst ..." -ForegroundColor Yellow
$cp = Wait-Probe
if (-not $cp -or $cp -notmatch "^PASS termux") {
    Write-Host "FAIL copy: $cp" -ForegroundColor Red
    exit 1
}

$catMsg = @"
In Termux workspace use exec_shell only. First response JSON only: {"tool":"exec_shell","args":{"command":"cat $dst","timeout_secs":30}}. Reply with file content only.
"@
Set-ProbeMessage $catMsg
Start-YoloProbe
Write-Host "Step 3: verify cat $dst ..." -ForegroundColor Yellow
$v = Wait-Probe
if ($v -match "COPY_SRC") {
    Write-Host "PASS copy_file verified" -ForegroundColor Green
    exit 0
}
Write-Host "FAIL verify: $v" -ForegroundColor Red
exit 1
