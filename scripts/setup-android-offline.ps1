# Prepare DeepSeek-Mobile for Android build without downloading SDK again.
# NDK and dioxus-cli still need internet when available.

$ErrorActionPreference = "Stop"
$ProjectRoot = Split-Path -Parent $PSScriptRoot

Write-Host "DeepSeek-Mobile Android offline prep"
Write-Host "Project: $ProjectRoot"

& "$ProjectRoot\tools\android\sync-sdk-from-system.ps1"

Write-Host ""
Write-Host "Checking Rust Android targets (install needs network if missing)..."
$targets = @(
    "aarch64-linux-android",
    "armv7-linux-androideabi",
    "x86_64-linux-android"
)
foreach ($t in $targets) {
    $installed = rustup target list --installed 2>$null | Select-String -SimpleMatch $t
    if ($installed) {
        Write-Host "  OK  $t"
    } else {
        Write-Host "  MISSING  $t  -> run: rustup target add $t"
    }
}

Write-Host ""
Write-Host "NDK: unpack to:"
Write-Host "  $ProjectRoot\tools\android\sdk\ndk\26.1.10909125"
Write-Host "Download size ~631 MB when online."

Write-Host ""
Write-Host "Dioxus CLI (when online):"
Write-Host "  cargo install dioxus-cli --version 0.7.9 --locked"

Write-Host ""
Write-Host "Then:"
Write-Host "  . $ProjectRoot\tools\android\env.ps1"
Write-Host "  dx build android"
