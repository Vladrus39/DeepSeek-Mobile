# First-time: build + install the debug APK on the connected phone (same as update-phone-apk, explicit name).
# Alias for onboarding docs — calls update-phone-apk.ps1.

param(
    [Parameter(ValueFromRemainingArguments = $true)]
    [string[]]$PassThrough
)

$scriptPath = Join-Path $PSScriptRoot "update-phone-apk.ps1"
& $scriptPath -Launch @PassThrough
exit $LASTEXITCODE
