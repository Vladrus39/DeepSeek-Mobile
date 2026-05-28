# Windows firewall + network rules so deepseek-pc-host mDNS reaches phones on the LAN.
# Run as Administrator. Safe to re-run (idempotent rules).
#
# Usage:
#   powershell -ExecutionPolicy Bypass -File .\scripts\enable-pc-host-mdns-windows.ps1
#   powershell -ExecutionPolicy Bypass -File .\scripts\enable-pc-host-mdns-windows.ps1 -TcpPort 8787

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
    Write-Host "[+] Created: $DisplayName ($Direction $Protocol $($LocalPort -join ','))" -ForegroundColor Cyan
}

Write-Host "=== DeepSeek PC Host — Windows LAN / mDNS firewall ===" -ForegroundColor Cyan

if (-not (Test-Admin)) {
    Write-Host "WARN: not running as Administrator — some rules may fail." -ForegroundColor Yellow
    Write-Host "Re-run: Start-Process powershell -Verb RunAs -ArgumentList '-ExecutionPolicy Bypass -File `"$PSCommandPath`"'"
}

if (-not $SkipTcp) {
    Ensure-FirewallRule `
        -DisplayName "DeepSeek PC Host TCP $TcpPort" `
        -Direction Inbound `
        -Protocol TCP `
        -LocalPort @($TcpPort)
}

if (-not $SkipMdns) {
    # mDNS / DNS-SD (Android NSD, Bonjour)
    Ensure-FirewallRule `
        -DisplayName "DeepSeek mDNS In (UDP 5353)" `
        -Direction Inbound `
        -Protocol UDP `
        -LocalPort @(5353)
    Ensure-FirewallRule `
        -DisplayName "DeepSeek mDNS Out (UDP 5353)" `
        -Direction Outbound `
        -Protocol UDP `
        -LocalPort @(5353)

    Write-Host "Enabling Network Discovery rules on Private profile (helps multicast)..." -ForegroundColor Gray
    try {
        Get-NetFirewallRule -DisplayGroup "Network Discovery" -ErrorAction Stop |
            Where-Object { $_.Profile -match "Private" -and $_.Enabled -eq "False" } |
            Enable-NetFirewallRule -ErrorAction SilentlyContinue
        Write-Host "[OK] Network Discovery rules enabled where possible" -ForegroundColor Green
    } catch {
        Write-Host "[WARN] Could not adjust Network Discovery rules: $_" -ForegroundColor Yellow
    }
}

Remove-Item Env:DEEPSEEK_PC_HOST_DISABLE_MDNS -ErrorAction SilentlyContinue
Write-Host ""
Write-Host "Next steps:" -ForegroundColor Cyan
Write-Host "  1. `$env:DEEPSEEK_PC_HOST_BIND = '0.0.0.0:$TcpPort'"
Write-Host "  2. cargo run -p deepseek-pc-host   (look for 'deepseek-pc-host mDNS: ... on <LAN-IP>:$TcpPort')"
Write-Host "  3. Phone: PC Host -> Scan LAN (mDNS)  — or manual http://<PC-IP>:$TcpPort"
Write-Host ""
Write-Host "PASS Windows mDNS/firewall prep complete." -ForegroundColor Green
