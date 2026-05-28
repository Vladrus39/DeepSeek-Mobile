# Push LAN MCP demo server config to the phone (same Wi-Fi as PC).
param(
    [string]$Device = "RFCNC0PWD4E",
    [string]$Package = "com.deepseek.mobile",
    [int]$Port = 3333,
    [string]$PcIp = ""
)

$ErrorActionPreference = "Stop"
$root = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$adb = Join-Path $root "tools\android\sdk\platform-tools\adb.exe"
if (-not (Test-Path $adb)) { $adb = "adb" }

if (-not $PcIp) {
    $line = (ipconfig | Select-String -Pattern "IPv4" | Select-Object -First 1).Line
    if ($line -match ":\s*([0-9.]+)") { $PcIp = $Matches[1] }
}
if (-not $PcIp) { throw "Could not detect PC IPv4; pass -PcIp" }

$url = "http://${PcIp}:$Port"
$mcp = @"
[
  {
    "name": "demo",
    "transport": "http_sse",
    "url": "$url",
    "enabled": true,
    "description": "LAN MCP demo server (scripts/mcp_demo_http_server.py)",
    "declared_tools": [
      {
        "name": "echo",
        "server": "demo",
        "description": "Echo back provided text.",
        "input_schema": {
          "type": "object",
          "properties": { "text": { "type": "string" } },
          "required": ["text"],
          "additionalProperties": false
        }
      }
    ]
  }
]
"@

$out = Join-Path $root "target\mcp-device.json"
New-Item -ItemType Directory -Force -Path (Split-Path $out) | Out-Null
$mcp | Out-File -Encoding utf8 $out

$adbArgs = @()
if ($Device) { $adbArgs += @("-s", $Device) }

& $adb @adbArgs push $out /data/local/tmp/mcp.json | Out-Null
& $adb @adbArgs shell run-as $Package cp /data/local/tmp/mcp.json files/deepseek-mobile/mcp.json
Write-Host "MCP config -> $url" -ForegroundColor Green
& $adb @adbArgs shell run-as $Package cat files/deepseek-mobile/mcp.json
