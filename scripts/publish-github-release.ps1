# Build release APK and publish a GitHub Release with the versioned APK asset.
#
# Requires: gh auth login, android SDK (tools/android/env.ps1), optional android/keystore.properties
#
# Usage:
#   .\scripts\publish-github-release.ps1
#   .\scripts\publish-github-release.ps1 -Tag v0.1.1 -NotesFile RELEASE_NOTES.md

param(
    [string]$Tag = "",
    [string]$NotesFile = "",
    [switch]$SkipTests,
    [switch]$SkipBuild,
    [string]$Repo = "Vladrus39/DeepSeek-Mobile"
)

$ErrorActionPreference = "Stop"
$ProjectRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
Set-Location $ProjectRoot

$version = (Select-String -Path "crates\mobile\Cargo.toml" -Pattern '^version = "([^"]+)"' |
    ForEach-Object { $_.Matches.Groups[1].Value } | Select-Object -First 1)
if (-not $version) { throw "Could not read version from crates/mobile/Cargo.toml" }
if (-not $Tag) { $Tag = "v$version" }

$buildScript = Join-Path $ProjectRoot "scripts\build-release-apk.ps1"
if (-not $SkipBuild) {
    $buildArgs = @()
    if ($SkipTests) { $buildArgs += "-SkipTests" }
    & $buildScript @buildArgs
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
}

$assetPath = Join-Path $ProjectRoot "dist\deepseek-mobile-$version.apk"
if (-not (Test-Path $assetPath)) {
    throw "Missing APK asset: $assetPath (run build-release-apk.ps1 first)"
}

$gh = Get-Command gh -ErrorAction SilentlyContinue
if (-not $gh) { throw "GitHub CLI (gh) not found. Install from https://cli.github.com/" }

$releaseArgs = @("release", "create", $Tag, $assetPath, "--repo", $Repo, "--title", "DeepSeek Mobile $Tag")
if ($NotesFile) {
    if (-not (Test-Path $NotesFile)) { throw "Notes file not found: $NotesFile" }
    $releaseArgs += @("--notes-file", $NotesFile)
} else {
    $releaseArgs += @("--generate-notes")
}

Write-Host "Publishing $Tag to $Repo ..." -ForegroundColor Cyan
& gh @releaseArgs
if ($LASTEXITCODE -ne 0) { throw "gh release create failed" }
Write-Host "Published: https://github.com/$Repo/releases/tag/$Tag" -ForegroundColor Green
