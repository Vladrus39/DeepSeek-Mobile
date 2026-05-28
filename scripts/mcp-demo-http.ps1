# Minimal MCP JSON-RPC HTTP server for local E2E.
#
# Exposes POST /mcp with:
# - tools/list -> returns one tool: echo
# - tools/call -> returns a text payload
#
# Usage:
#   powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\mcp-demo-http.ps1 -Bind 0.0.0.0 -Port 3333

param(
    [string]$Bind = "0.0.0.0",
    [int]$Port = 3333
)

$ErrorActionPreference = "Stop"
$root = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$py = "python"

$server = Join-Path $root "scripts\mcp_demo_http_server.py"
if (-not (Test-Path $server)) {
    throw "Missing server script: $server"
}

Write-Host "Starting MCP demo server on http://$Bind`:$Port/mcp" -ForegroundColor Cyan
& $py $server --host $Bind --port $Port

