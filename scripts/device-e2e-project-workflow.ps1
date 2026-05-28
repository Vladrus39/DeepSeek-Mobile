# E2E project workflow in Termux deepseek-project (YOLO probe, Agent config restored after).
# Steps: create test_e2e_project/hello.txt, append WORLD, exec_shell ls/git.
# Usage: . .\tools\android\env.ps1; .\scripts\device-e2e-project-workflow.ps1 -Serial RFCNC0PWD4E [-Build]

param(
    [string]$Serial = "",
    [switch]$Build
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

$pkg = "com.deepseek.mobile"
$data = "files/deepseek-mobile"
$results = [ordered]@{}

Push-Location $ProjectRoot
try {
    if ($Build) {
        if ($Serial) { dx build --android --package deepseek-mobile --device $Serial 2>&1 | Out-Host }
        else { dx build --android --package deepseek-mobile 2>&1 | Out-Host }
        $apk = Join-Path $ProjectRoot "target\dx\deepseek-mobile\debug\android\app\app\build\outputs\apk\debug\app-debug.apk"
        if (Test-Path $apk) { Invoke-Adb @("install", "-r", $apk) | Out-Host }
    }
} finally { Pop-Location }

function Set-ProbeMessage([string]$Text) {
    $tmp = Join-Path $env:TEMP "deepseek-probe-msg.txt"
    Set-Content -Path $tmp -Value $Text -Encoding UTF8
    Invoke-Adb @("push", $tmp, "/data/local/tmp/deepseek-probe-msg.txt") | Out-Null
    Invoke-Adb @("shell", "run-as", $pkg, "cp", "/data/local/tmp/deepseek-probe-msg.txt", "$data/.agent_turn_probe_message") | Out-Null
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

function Verify-File([string]$RelPath, [string]$MustContain) {
    $msg = "Use exec_shell only. First response JSON only: {""tool"":""exec_shell"",""args"":{""command"":""cat $RelPath"",""timeout_secs"":30}}. Reply with file contents only."
    Set-ProbeMessage $msg
    Start-YoloProbe
    $r = Wait-Probe
    if (-not $r) { return "FAIL no probe result" }
    if ($r -match '^PASS' -and $r -match $MustContain) { return "PASS $r" }
    return "FAIL probe_or_content_missing $r"
}

# Ensure Agent mode (not Yolo) in persisted config after automation
function Restore-AgentConfig {
    $cfgRaw = Invoke-Adb @("shell", "run-as", $pkg, "cat", "$data/config.json")
    $cfgText = ($cfgRaw | Out-String).Trim()
    if ($cfgText -match "No such file") { return }
    $updated = $cfgText -replace '"execution_mode"\s*:\s*"Yolo"', '"execution_mode":"Agent"'
    if ($updated -ne $cfgText) {
        $tmp = Join-Path $env:TEMP "deepseek-cfg-agent.json"
        Set-Content -Path $tmp -Value $updated -Encoding UTF8
        Invoke-Adb @("push", $tmp, "/data/local/tmp/deepseek-cfg-agent.json") | Out-Null
        Invoke-Adb @("shell", "run-as", $pkg, "cp", "/data/local/tmp/deepseek-cfg-agent.json", "$data/config.json") | Out-Null
    }
    Invoke-Adb @("shell", "run-as", $pkg, "rm", "-f", "$data/.agent_turn_probe_yolo") | Out-Null
}

Write-Host "=== Project workflow E2E (YOLO probes, Agent config after) ===" -ForegroundColor Cyan
Invoke-Adb @("logcat", "-c") -AllowFail | Out-Null

$stepA = @'
In Termux workspace use write_file only. First response must be exactly: {"tool":"write_file","args":{"path":"test_e2e_project/hello.txt","content":"HELLO_E2E"}}. Then reply HELLO_OK only.
'@
Set-ProbeMessage $stepA
Start-YoloProbe
Write-Host "Step A: create hello.txt (foreground ~2 min)..." -ForegroundColor Yellow
$results["step_a_write"] = Wait-Probe
$logA = (Invoke-Adb @("logcat", "-d", "-v", "time", "-t", "12000") | Out-String)
$hitsA = @($logA | Select-String -Pattern "RUN_COMMAND|RunCommandService|DeepSeekTermuxBridge|termux_run_command").Count
$results["step_a_run_command"] = if ($hitsA -gt 0) { "PASS hits=$hitsA" } else { "WARN no RUN_COMMAND in last logcat window" }
$results["step_a_verify"] = Verify-File "test_e2e_project/hello.txt" "HELLO_E2E"

$stepB = @'
In Termux workspace use write_file to replace test_e2e_project/hello.txt with exactly two lines: HELLO_E2E and WORLD_E2E. JSON only first: {"tool":"write_file","args":{"path":"test_e2e_project/hello.txt","content":"HELLO_E2E`nWORLD_E2E"}}. Reply WORLD_OK.
'@
Set-ProbeMessage $stepB
Start-YoloProbe
Write-Host "Step B: edit hello.txt..." -ForegroundColor Yellow
$results["step_b_write"] = Wait-Probe
$results["step_b_verify"] = Verify-File "test_e2e_project/hello.txt" "WORLD_E2E"

$stepC = @'
Use exec_shell only. First JSON: {"tool":"exec_shell","args":{"command":"pwd && ls -la test_e2e_project && git status 2>/dev/null || true","timeout_secs":45}}. Reply with command output only.
'@
Set-ProbeMessage $stepC
Start-YoloProbe
Write-Host "Step C: ls/git..." -ForegroundColor Yellow
$results["step_c_shell"] = Wait-Probe

Restore-AgentConfig

Write-Host "`n=== Project workflow summary ===" -ForegroundColor Cyan
foreach ($k in $results.Keys) {
    $c = if ($results[$k] -match "PASS") { "Green" } elseif ($results[$k] -match "FAIL") { "Red" } else { "Yellow" }
    Write-Host ("  {0,-22} {1}" -f $k, $results[$k]) -ForegroundColor $c
}

$fail = @($results.Values | Where-Object { $_ -match "FAIL" }).Count
if ($fail -gt 0) { exit 1 }
