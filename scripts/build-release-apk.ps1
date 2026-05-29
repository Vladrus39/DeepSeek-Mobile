# Build a release Android APK and copy it to dist/ with the GitHub Release asset name.
#
# Optional signing: copy android/keystore.properties.example to android/keystore.properties
# and point storeFile at your release keystore (never commit secrets).
#
# Usage (repo root):
#   . .\tools\android\env.ps1
#   .\scripts\build-release-apk.ps1
#   .\scripts\build-release-apk.ps1 -SkipTests
#
# CI note:
#   If tools/android/sdk is not present, this script uses ANDROID_HOME / ANDROID_SDK_ROOT
#   from the runner environment.

param(
    [switch]$SkipTests,
    [string]$Serial = ""
)

$ErrorActionPreference = "Stop"
$ProjectRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
Set-Location $ProjectRoot

$version = (Select-String -Path "crates\mobile\Cargo.toml" -Pattern '^version = "([^"]+)"' |
    ForEach-Object { $_.Matches.Groups[1].Value } | Select-Object -First 1)
if (-not $version) { throw "Could not read version from crates/mobile/Cargo.toml" }

$assetName = "deepseek-mobile-$version.apk"
$releaseApkRel = "target\dx\deepseek-mobile\release\android\app\app\build\outputs\apk\release\app-release.apk"
$releaseApkPath = Join-Path $ProjectRoot $releaseApkRel
$distDir = Join-Path $ProjectRoot "dist"
$distApk = Join-Path $distDir $assetName

$envScript = Join-Path $ProjectRoot "tools\android\env.ps1"
$localAdb = Join-Path $ProjectRoot "tools\android\sdk\platform-tools\adb.exe"
if ((Test-Path $envScript) -and (Test-Path $localAdb)) {
    . $envScript
} else {
    $sdkRoot = if ($env:ANDROID_HOME) { $env:ANDROID_HOME } elseif ($env:ANDROID_SDK_ROOT) { $env:ANDROID_SDK_ROOT } else { "" }
    if (-not $sdkRoot) {
        throw "No Android SDK found. Run tools/android/sync-sdk-from-system.ps1 locally or set ANDROID_HOME/ANDROID_SDK_ROOT in CI."
    }
    $env:ANDROID_HOME = $sdkRoot
    $env:ANDROID_SDK_ROOT = $sdkRoot
    $platformTools = Join-Path $sdkRoot "platform-tools"
    if (Test-Path $platformTools) {
        $env:PATH = "$platformTools;$env:PATH"
    }
    Write-Host "Using Android SDK from environment: $sdkRoot" -ForegroundColor Cyan
}

if (-not $SkipTests) {
    Write-Host "Running cargo test --workspace ..." -ForegroundColor Cyan
    cargo +stable-x86_64-pc-windows-msvc test --workspace
    if ($LASTEXITCODE -ne 0) { throw "cargo test failed" }
}

Write-Host "Building release APK (dx) version=$version ..." -ForegroundColor Cyan
if ($Serial) {
    dx build --android --package deepseek-mobile --release --device $Serial --verbose
} else {
    dx build --android --package deepseek-mobile --release --verbose
}
if ($LASTEXITCODE -ne 0) { throw "dx build --release failed" }

if (-not (Test-Path $releaseApkPath)) {
    throw "Release APK not found: $releaseApkPath"
}

New-Item -ItemType Directory -Force -Path $distDir | Out-Null
Copy-Item -Force $releaseApkPath $distApk
Write-Host "Release APK: $distApk" -ForegroundColor Green
Write-Host "GitHub asset name: $assetName" -ForegroundColor Green
