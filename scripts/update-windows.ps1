# Safe one-command updater for an existing Windows checkout.
#
# From inside the repo:
#   powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\update-windows.ps1
#
# Also update the phone APK after git pull:
#   powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\update-windows.ps1 -PhoneApk -Serial RFCNC0PWD4E -Launch

param(
    [switch]$Check,
    [switch]$AllowDirty,
    [switch]$PhoneApk,
    [string]$Serial = "",
    [switch]$SkipTests,
    [switch]$Launch
)

$ErrorActionPreference = "Stop"

$RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
Set-Location $RepoRoot

if (-not (Test-Path ".git")) {
    throw "Not a git checkout: $RepoRoot"
}

$dirty = git status --porcelain
if ($dirty -and -not $AllowDirty) {
    Write-Host "Local changes detected. Update stopped to avoid overwriting work:" -ForegroundColor Yellow
    $dirty
    Write-Host ""
    Write-Host "Commit/stash your changes, or rerun with -AllowDirty if you know what you are doing."
    exit 2
}

$branch = (git branch --show-current).Trim()
if (-not $branch) {
    throw "Detached HEAD is not supported by the safe updater."
}

Write-Host "Fetching origin..." -ForegroundColor Cyan
git fetch origin

Write-Host "Updating $branch with fast-forward only..." -ForegroundColor Cyan
git pull --ff-only origin $branch

if (Test-Path ".\tools\android\env.ps1") {
    Write-Host "Repo-local Android env is available: . .\tools\android\env.ps1" -ForegroundColor Gray
}

if ($Check) {
    Write-Host "Running workspace check..." -ForegroundColor Cyan
    cargo +stable-x86_64-pc-windows-msvc check --workspace --all-targets
}

Write-Host ""
Write-Host "DeepSeek-Mobile updated." -ForegroundColor Green
Write-Host "Current commit:"
git log -1 --oneline

if ($PhoneApk) {
    Write-Host ""
    Write-Host "Building and installing Android APK ..." -ForegroundColor Cyan
    $phoneArgs = @("-SkipPull")
    if ($Serial) { $phoneArgs += @("-Serial", $Serial) }
    if ($SkipTests) { $phoneArgs += "-SkipTests" }
    if ($Launch) { $phoneArgs += "-Launch" }
    & (Join-Path $RepoRoot "scripts\update-phone-apk.ps1") @phoneArgs
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
}
