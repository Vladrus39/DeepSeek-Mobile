# One-command Windows installer for the source checkout.
#
# Recommended public command:
#   powershell -NoProfile -ExecutionPolicy Bypass -Command "irm https://raw.githubusercontent.com/Vladrus39/DeepSeek-Mobile/main/scripts/install-windows.ps1 | iex"

param(
    [string]$Dir = (Join-Path $HOME "DeepSeek-Mobile"),
    [string]$Repo = "https://github.com/Vladrus39/DeepSeek-Mobile.git",
    [string]$Branch = "main",
    [switch]$SkipUpdate
)

$ErrorActionPreference = "Stop"

function Require-Command {
    param([string]$Name, [string]$InstallHint)
    if (-not (Get-Command $Name -ErrorAction SilentlyContinue)) {
        throw "$Name is required. $InstallHint"
    }
}

Require-Command git "Install Git for Windows: winget install --id Git.Git -e"

$target = [System.IO.Path]::GetFullPath($Dir)
$parent = Split-Path -Parent $target
if ($parent) {
    New-Item -ItemType Directory -Force -Path $parent | Out-Null
}

if (Test-Path $target) {
    if (-not (Test-Path (Join-Path $target ".git"))) {
        throw "Target exists but is not a git checkout: $target"
    }
    Write-Host "Existing checkout found: $target" -ForegroundColor Yellow
} else {
    Write-Host "Cloning DeepSeek-Mobile into $target" -ForegroundColor Cyan
    git clone --branch $Branch $Repo $target
}

Set-Location $target

if (-not $SkipUpdate) {
    & (Join-Path $target "scripts\update-windows.ps1")
}

Write-Host ""
Write-Host "DeepSeek-Mobile is ready at:" -ForegroundColor Green
Write-Host "  $target"
Write-Host ""
Write-Host "Next:"
Write-Host "  cd `"$target`""
Write-Host "  . .\tools\android\env.ps1"
Write-Host "  cargo +stable-x86_64-pc-windows-msvc check --workspace --all-targets"
Write-Host ""
Write-Host "With a USB-debugging phone connected:"
Write-Host "  dx build --android --package deepseek-mobile --device <serial> --verbose"
