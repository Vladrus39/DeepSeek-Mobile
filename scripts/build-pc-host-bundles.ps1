# Build deepseek-pc-host release binaries and copy into tools/pc-host/bin for pairing ZIP embedding.
param(
    [string]$Profile = "release"
)

$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent $PSScriptRoot
Set-Location $Root

Write-Host "Building deepseek-pc-host ($Profile)..."
cargo build -p deepseek-pc-host --$Profile
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

$OutRoot = Join-Path $Root "tools\pc-host\bin"
$null = New-Item -ItemType Directory -Force -Path $OutRoot

$Release = Join-Path $Root "target\$Profile"
$WinDir = Join-Path $OutRoot "windows-x86_64"
$LinuxDir = Join-Path $OutRoot "linux-x86_64"
$null = New-Item -ItemType Directory -Force -Path $WinDir, $LinuxDir

Copy-Item -Force (Join-Path $Release "deepseek-pc-host.exe") (Join-Path $WinDir "deepseek-pc-host.exe")
Copy-Item -Force (Join-Path $Release "deepseek-pc-host.exe") (Join-Path $OutRoot "deepseek-pc-host.exe")

if (Test-Path (Join-Path $Release "deepseek-pc-host")) {
    Copy-Item -Force (Join-Path $Release "deepseek-pc-host") (Join-Path $LinuxDir "deepseek-pc-host")
    Copy-Item -Force (Join-Path $Release "deepseek-pc-host") (Join-Path $OutRoot "deepseek-pc-host")
}

Write-Host "Copied host binaries to $OutRoot"
Write-Host "Pairing ZIP export will embed these when discover_pc_host_binaries finds them."
