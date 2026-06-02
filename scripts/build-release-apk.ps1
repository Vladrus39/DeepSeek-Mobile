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
    [switch]$AllowUnsigned,
    [string]$Serial = ""
)

$ErrorActionPreference = "Stop"
$ProjectRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
Set-Location $ProjectRoot

$version = (Select-String -Path "crates\mobile\Cargo.toml" -Pattern '^version = "([^"]+)"' |
    ForEach-Object { $_.Matches.Groups[1].Value } | Select-Object -First 1)
if (-not $version) { throw "Could not read version from crates/mobile/Cargo.toml" }

$assetName = "deepseek-mobile-$version.apk"
$releaseApkDirRel = "target\dx\deepseek-mobile\release\android\app\app\build\outputs\apk\release"
$releaseApkRel = "$releaseApkDirRel\app-release.apk"
$releaseApkPath = Join-Path $ProjectRoot $releaseApkRel
$dxReleaseApp = Join-Path $ProjectRoot "target\dx\deepseek-mobile\release\android\app"
$repoKeystoreProps = Join-Path $ProjectRoot "android\keystore.properties"
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

$gradlew = Join-Path $ProjectRoot "target\dx\deepseek-mobile\release\android\app\gradlew.bat"
if (Test-Path $gradlew) {
    if (Test-Path $repoKeystoreProps) {
        Copy-Item -Force $repoKeystoreProps (Join-Path (Split-Path $gradlew) "keystore.properties")
    }
    Write-Host "Running Gradle clean assembleRelease (fresh APK with current jniLibs)..." -ForegroundColor Cyan
    Push-Location (Split-Path $gradlew)
    & $gradlew clean assembleRelease
    if ($LASTEXITCODE -ne 0) { Pop-Location; throw "gradlew clean assembleRelease failed" }
    Pop-Location
}

if (-not (Test-Path $releaseApkPath)) {
    $gradlewLegacy = Join-Path $dxReleaseApp "gradlew.bat"
    if (Test-Path $gradlewLegacy) {
        if (Test-Path $repoKeystoreProps) {
            Copy-Item -Force $repoKeystoreProps (Join-Path $dxReleaseApp "keystore.properties")
        }
        Write-Host "Running Gradle assembleRelease (dx bundle did not emit release APK)..." -ForegroundColor Cyan
        Push-Location $dxReleaseApp
        & $gradlewLegacy assembleRelease
        if ($LASTEXITCODE -ne 0) { Pop-Location; throw "gradlew assembleRelease failed" }
        Pop-Location
    }
}
if (-not (Test-Path $releaseApkPath)) {
    $signed = Join-Path $ProjectRoot "$releaseApkDirRel\app-release.apk"
    $unsigned = Join-Path $ProjectRoot "$releaseApkDirRel\app-release-unsigned.apk"
    if (Test-Path $signed) {
        $releaseApkPath = $signed
    } elseif (Test-Path $unsigned) {
        if (-not $AllowUnsigned) {
            throw "Only an unsigned release APK was produced. Configure android/keystore.properties for public releases, or pass -AllowUnsigned for local diagnostics only."
        }
        $releaseApkPath = $unsigned
        Write-Host "Using unsigned release APK for local diagnostics only." -ForegroundColor Yellow
    }
}
if (-not (Test-Path $releaseApkPath)) {
    throw "Release APK not found under target/dx/deepseek-mobile/release"
}

$buildToolsRoot = Join-Path $env:ANDROID_SDK_ROOT "build-tools"
$aapt = Get-ChildItem -Path $buildToolsRoot -Filter "aapt.exe" -Recurse -ErrorAction SilentlyContinue |
    Sort-Object FullName -Descending |
    Select-Object -First 1
if (-not $aapt) {
    throw "Android aapt.exe not found under $buildToolsRoot; cannot validate APK ABI compatibility."
}
$badging = & $aapt.FullName dump badging $releaseApkPath
if ($LASTEXITCODE -ne 0) { throw "aapt dump badging failed for $releaseApkPath" }
$nativeCodeLine = $badging | Where-Object { $_ -like "native-code:*" } | Select-Object -First 1
if (-not $nativeCodeLine -or $nativeCodeLine -notmatch "'arm64-v8a'") {
    throw "Release APK is missing arm64-v8a native code. Refusing to publish a phone-incompatible APK. native-code=$nativeCodeLine"
}
Write-Host "APK ABI check passed: $nativeCodeLine" -ForegroundColor Green

New-Item -ItemType Directory -Force -Path $distDir | Out-Null
Copy-Item -Force $releaseApkPath $distApk
Write-Host "Release APK: $distApk" -ForegroundColor Green
Write-Host "GitHub asset name: $assetName" -ForegroundColor Green
