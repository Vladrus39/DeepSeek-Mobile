# Windows firewall + network rules so deepseek-pc-host mDNS reaches phones on the LAN.
# Run as Administrator. Safe to re-run (idempotent rules).
#
# Usage (any cwd):
#   powershell -ExecutionPolicy Bypass -File "C:\path\to\DeepSeek-Mobile\scripts\enable-pc-host-mdns-windows.ps1"
# Or double-click repo root: enable-pc-host-firewall.cmd (Run as administrator)
param(
    [int]$TcpPort = 8787,
    [switch]$SkipTcp,
    [switch]$SkipMdns
)

$ErrorActionPreference = "Stop"

function Test-Admin {
    $identity = [Security.Principal.WindowsIdentity]::GetCurrent()
    $principal = New-Object Security.Principal.WindowsPrincipal($identity)
    return $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
}

function Ensure-FirewallRule {
    param(
        [string]$DisplayName,
        [string]$Direction,
        [string]$Protocol,
        [int[]]$LocalPort,
        [string]$Profile = "Private"
    )
    $existing = Get-NetFirewallRule -DisplayName $DisplayName -ErrorAction SilentlyContinue
    if ($existing) {
        Write-Host "[OK] Rule exists: $DisplayName" -ForegroundColor Green
        return
    }
    New-NetFirewallRule `
        -DisplayName $DisplayName `
        -Direction $Direction `
        -Action Allow `
        -Protocol $Protocol `
        -LocalPort $LocalPort `
        -Profile $Profile | Out-Null
    Write-Host "[+] Created: $DisplayName" -ForegroundColor Cyan
}

Write-Host "=== DeepSeek PC Host - Windows LAN / mDNS firewall ===" -ForegroundColor Cyan

if (-not (Test-Admin)) {
    Write-Host "WARN: not running as Administrator - some rules may fail." -ForegroundColor Yellow
}

if (-not $SkipTcp) {
    Ensure-FirewallRule -DisplayName "DeepSeek PC Host TCP $TcpPort" -Direction Inbound -Protocol TCP -LocalPort @($TcpPort)
}

if (-not $SkipMdns) {
    Ensure-FirewallRule -DisplayName "DeepSeek mDNS In (UDP 5353)" -Direction Inbound -Protocol UDP -LocalPort @(5353)
    Ensure-FirewallRule -DisplayName "DeepSeek mDNS Out (UDP 5353)" -Direction Outbound -Protocol UDP -LocalPort @(5353)
}

Remove-Item Env:DEEPSEEK_PC_HOST_DISABLE_MDNS -ErrorAction SilentlyContinue
Write-Host "PASS Windows mDNS/firewall prep complete." -ForegroundColor Green
