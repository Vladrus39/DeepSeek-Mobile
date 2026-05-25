# Re-copy a minimal SDK slice from the system install into this repo (no internet).
param(
    [string]$SourceSdk = "$env:LOCALAPPDATA\Android\Sdk"
)

$ErrorActionPreference = "Stop"
$ProjectRoot = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path
$DstSdk = Join-Path $ProjectRoot "tools\android\sdk"

if (-not (Test-Path $SourceSdk)) {
    Write-Error "System SDK not found at: $SourceSdk"
}

$components = @(
    @{ Name = "platform-tools"; Src = "platform-tools"; Dst = "platform-tools" },
    @{ Name = "build-tools 35.0.0"; Src = "build-tools\35.0.0"; Dst = "build-tools\35.0.0" },
    @{ Name = "platform android-36"; Src = "platforms\android-36"; Dst = "platforms\android-36" }
)

New-Item -ItemType Directory -Force -Path $DstSdk | Out-Null

foreach ($item in $components) {
    $from = Join-Path $SourceSdk $item.Src
    $to = Join-Path $DstSdk $item.Dst
    if (-not (Test-Path $from)) {
        Write-Warning "Skip missing: $($item.Name) ($from)"
        continue
    }
    New-Item -ItemType Directory -Force -Path (Split-Path $to -Parent) | Out-Null
    Write-Host "Copying $($item.Name)..."
    robocopy $from $to /E /NFL /NDL /NJH /NJS /nc /ns /np | Out-Null
}

$total = (Get-ChildItem $DstSdk -Recurse -File | Measure-Object Length -Sum).Sum
Write-Host "Local SDK at $DstSdk — $([math]::Round($total/1MB, 1)) MB"
