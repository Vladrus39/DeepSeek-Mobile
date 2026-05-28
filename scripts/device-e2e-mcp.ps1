# E2E: MCP echo tool via agent_turn_probe (requires MCP HTTP server on PC).
param(
    [string]$Device = "RFCNC0PWD4E",
    [string]$Package = "com.deepseek.mobile",
    [int]$McpPort = 3333,
    [int]$TimeoutSec = 90
)

$ErrorActionPreference = "Continue"
$root = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path

# Ensure demo server is reachable (start in background if missing).
try {
    $null = Invoke-WebRequest -UseBasicParsing "http://127.0.0.1:$McpPort/mcp" -Method Post `
        -ContentType "application/json" `
        -Body '{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}' `
        -TimeoutSec 2
} catch {
    Write-Host "Starting MCP demo server on port $McpPort..." -ForegroundColor Cyan
    Start-Process -WindowStyle Hidden python -ArgumentList @(
        (Join-Path $root "scripts\mcp_demo_http_server.py"),
        "--host", "0.0.0.0",
        "--port", "$McpPort"
    )
    Start-Sleep 2
}

& (Join-Path $root "scripts\push-mcp-config-to-device.ps1") -Device $Device -Port $McpPort

$adb = Join-Path $root "tools\android\sdk\platform-tools\adb.exe"
$AdbArgs = @("-s", $Device)

function Invoke-Adb([string[]]$Cmd) {
    & $adb @AdbArgs @Cmd 2>&1
}

Invoke-Adb @("shell", "am", "start", "-n", "$Package/dev.dioxus.main.MainActivity") | Out-Null
Start-Sleep 3

$msg = 'Use MCP tool mcp__demo__echo with args {"text":"MCP_E2E"}. Reply with only the tool output text.'
$path = Join-Path $root "target\mcp-probe-message.txt"
New-Item -ItemType Directory -Force -Path (Split-Path $path) | Out-Null
$msg | Out-File -Encoding utf8 -NoNewline $path

$base = "files/deepseek-mobile"
Invoke-Adb @("push", $path, "/data/local/tmp/mcp-probe-message.txt") | Out-Null
Invoke-Adb @("shell", "run-as", $Package, "cp", "/data/local/tmp/mcp-probe-message.txt", "$base/.agent_turn_probe_message") | Out-Null
Invoke-Adb @("shell", "run-as", $Package, "rm", "-f", "$base/.agent_turn_probe_result") | Out-Null
Invoke-Adb @("shell", "run-as", $Package, "touch", "$base/.agent_turn_probe_yolo") | Out-Null
Invoke-Adb @("shell", "run-as", $Package, "touch", "$base/.agent_turn_probe_requested") | Out-Null

$deadline = (Get-Date).AddSeconds($TimeoutSec)
$result = $null
while ((Get-Date) -lt $deadline) {
    Start-Sleep 3
    $raw = Invoke-Adb @("shell", "run-as", $Package, "cat", "$base/.agent_turn_probe_result")
    $txt = ($raw | Out-String).Trim()
    if ($txt -and $txt -notmatch "No such file") {
        $result = $txt
        break
    }
}

if (-not $result) {
    Write-Host "FAIL timeout MCP probe" -ForegroundColor Red
    exit 1
}

Write-Host $result
if ($result -match "MCP_E2E") { exit 0 }
if ($result -match "^PASS") { exit 0 }
exit 1
