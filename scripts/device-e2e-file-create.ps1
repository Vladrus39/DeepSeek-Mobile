<# 
E2E: create + verify a real file inside Termux workspace via the app agent.
Usage:
  . .\tools\android\env.ps1
  .\scripts\device-e2e-file-create.ps1 -Serial RFCNC0PWD4E
#>

param(
  [string]$Serial = "",
  [switch]$Build,
  [switch]$RestoreAgentMode
)

$ErrorActionPreference = "Continue"
$ProjectRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$adb = Join-Path $ProjectRoot "tools\android\sdk\platform-tools\adb.exe"
if (-not (Test-Path $adb)) { $adb = "adb" }

function Invoke-Adb([string[]]$AdbArgs) {
  if ($Serial) { return & $adb -s $Serial @AdbArgs 2>&1 }
  return & $adb @AdbArgs 2>&1
}

$pkg  = "com.deepseek.mobile"
$data = "files/deepseek-mobile"

$writeMessage = "In the Termux project workspace, use real tools only. First response must be exactly: {""tool"":""write_file"",""args"":{""path"":""test_verify_e2e.txt"",""content"":""HELLO_E2E""}}. After tool result reply exactly: HELLO_E2E_OK"
$verifyMessage = "In the Termux project workspace use exec_shell only. First response must be exactly: {""tool"":""exec_shell"",""args"":{""command"":""cat test_verify_e2e.txt"",""timeout_secs"":30}}. After tool result reply with the file contents only."

Push-Location $ProjectRoot
try {
  if ($Build) {
    if ($Serial) { dx build --android --package deepseek-mobile --device $Serial 2>&1 | Out-Host }
    else { dx build --android --package deepseek-mobile 2>&1 | Out-Host }
    $apk = Join-Path $ProjectRoot "target\dx\deepseek-mobile\debug\android\app\app\build\outputs\apk\debug\app-debug.apk"
    if (Test-Path $apk) { Invoke-Adb @("install","-r",$apk) | Out-Host }
  }
} finally { Pop-Location }

function Set-ProbeMessage([string]$Text) {
  $tmp = Join-Path $env:TEMP "deepseek-probe-message.txt"
  Set-Content -Path $tmp -Value $Text -Encoding UTF8
  Invoke-Adb @("push", $tmp, "/data/local/tmp/deepseek-probe-message.txt") | Out-Null
  Invoke-Adb @("shell","run-as",$pkg,"cp","/data/local/tmp/deepseek-probe-message.txt",("$data/.agent_turn_probe_message")) | Out-Null
}

function Start-Probe {
  Invoke-Adb @("shell","run-as",$pkg,"rm","-f",("$data/.agent_turn_probe_result")) | Out-Null
  Invoke-Adb @("shell","run-as",$pkg,"touch",("$data/.agent_turn_probe_yolo")) | Out-Null
  Invoke-Adb @("shell","run-as",$pkg,"touch",("$data/.agent_turn_probe_requested")) | Out-Null
  Invoke-Adb @("shell","am","start","-n",("$pkg/dev.dioxus.main.MainActivity")) | Out-Null
}

function Wait-ProbeResult([int]$Seconds = 150) {
  $deadline = (Get-Date).AddSeconds($Seconds)
  while ((Get-Date) -lt $deadline) {
    Start-Sleep -Seconds 4
    $raw = Invoke-Adb @("shell","run-as",$pkg,"cat",("$data/.agent_turn_probe_result"))
    $result = ($raw | Out-String).Trim()
    if ($result -and $result -notmatch "No such file") { return $result }
  }
  return $null
}

function Get-RunCommandHits {
  $log = Invoke-Adb @("logcat","-d","-v","time","-t","8000")
  $text = ($log | Out-String)
  return @($text | Select-String -Pattern "RUN_COMMAND|RunCommandService|DeepSeekTermuxBridge|termux_run_command").Count
}

Write-Host "=== E2E write_file (YOLO) ===" -ForegroundColor Cyan
Invoke-Adb @("logcat","-c") | Out-Null
Set-ProbeMessage $writeMessage
Start-Probe
$writeResult = Wait-ProbeResult
$hits = Get-RunCommandHits

Write-Host "write_result=$writeResult" -ForegroundColor Yellow
Write-Host "run_command_hits=$hits" -ForegroundColor Cyan

Write-Host "`n=== E2E verify (exec_shell cat) ===" -ForegroundColor Cyan
Set-ProbeMessage $verifyMessage
Start-Probe
$verifyResult = Wait-ProbeResult

$fileOk = ($verifyResult -match "HELLO_E2E")
Write-Host "verify_result=$verifyResult" -ForegroundColor $(if ($fileOk) { "Green" } else { "Red" })

$overall = "FAIL"
if ($fileOk -and $hits -gt 0) { $overall = "PASS" }
elseif ($fileOk) { $overall = "PARTIAL" }
Write-Host "`n=== SUMMARY: $overall ===" -ForegroundColor $(if ($overall -eq "PASS") { "Green" } elseif ($overall -eq "PARTIAL") { "Yellow" } else { "Red" })

if ($RestoreAgentMode) {
  Invoke-Adb @("shell","run-as",$pkg,"rm","-f",("$data/.agent_turn_probe_yolo")) | Out-Null
}

if ($overall -ne "PASS") { exit 1 }

