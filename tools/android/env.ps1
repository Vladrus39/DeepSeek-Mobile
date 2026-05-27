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
    $NdkBin = Join-Path $env:ANDROID_NDK_HOME "toolchains\llvm\prebuilt\windows-x86_64\bin"
    if (Test-Path $NdkBin) {
        $env:PATH = "$NdkBin;$env:PATH"

        # Cargo/cc-rs (ring, rustls, etc.) does not get the linker from Dioxus.
        # Keep direct Android target checks working after sourcing this file:
        #   cargo check --target aarch64-linux-android
        $env:CC_aarch64_linux_android = Join-Path $NdkBin "aarch64-linux-android28-clang.cmd"
        $env:CXX_aarch64_linux_android = Join-Path $NdkBin "aarch64-linux-android28-clang++.cmd"
        $env:AR_aarch64_linux_android = Join-Path $NdkBin "llvm-ar.exe"
        $env:CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER = Join-Path $NdkBin "aarch64-linux-android28-clang.cmd"
        $env:CARGO_TARGET_AARCH64_LINUX_ANDROID_AR = Join-Path $NdkBin "llvm-ar.exe"
    }
}

Write-Host "ANDROID_SDK_ROOT=$env:ANDROID_SDK_ROOT"
if ($env:ANDROID_NDK_HOME) { Write-Host "ANDROID_NDK_HOME=$env:ANDROID_NDK_HOME" }
