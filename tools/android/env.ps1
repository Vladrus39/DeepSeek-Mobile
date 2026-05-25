# Project-local Android toolchain (DeepSeek-Mobile only).
# Usage: . .\tools\android\env.ps1

$ProjectRoot = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path
$LocalSdk = Join-Path $ProjectRoot "tools\android\sdk"

if (-not (Test-Path (Join-Path $LocalSdk "platform-tools\adb.exe"))) {
    Write-Error "Local SDK missing. Run: .\tools\android\sync-sdk-from-system.ps1"
    exit 1
}

$env:DEEPSEEK_ANDROID_SDK = $LocalSdk
$env:ANDROID_SDK_ROOT = $LocalSdk
$env:ANDROID_HOME = $LocalSdk
$env:PATH = "$(Join-Path $LocalSdk 'platform-tools');$env:PATH"

if (Test-Path (Join-Path $LocalSdk "ndk\26.1.10909125")) {
    $env:ANDROID_NDK_HOME = Join-Path $LocalSdk "ndk\26.1.10909125"
    $env:NDK_HOME = $env:ANDROID_NDK_HOME
}

Write-Host "ANDROID_SDK_ROOT=$env:ANDROID_SDK_ROOT"
