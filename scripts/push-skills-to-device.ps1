# Push skills-bundle to the connected Android device.
param(
    [string]$Device = "",
    [string]$Package = "com.deepseek.mobile"
)

$ErrorActionPreference = "Stop"
$root = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$adb = Join-Path $root "tools\android\sdk\platform-tools\adb.exe"
if (-not (Test-Path $adb)) { $adb = "adb" }

$skillsSrc = Join-Path $root "skills-bundle\skills"
if (-not (Test-Path $skillsSrc)) {
    throw "Missing skills bundle at $skillsSrc"
}

$adbArgs = @()
if ($Device) { $adbArgs += @("-s", $Device) }

function Invoke-Adb {
    param([string[]]$Cmd)
    & $adb @adbArgs @Cmd
    if ($LASTEXITCODE -ne 0) { throw "adb failed: $($Cmd -join ' ')" }
}

$devices = (Invoke-Adb @("devices")) -split "`n" | Where-Object { $_ -match "`tdevice$" }
if (-not $devices -and -not $Device) {
    throw "No adb device in 'device' state"
}

$externalDst = "/sdcard/Android/data/$Package/files/deepseek-mobile/skills"
Invoke-Adb @("shell", "mkdir", "-p", $externalDst)
Invoke-Adb @("push", "$skillsSrc/.", $externalDst)

Write-Host "Pushed skills to $externalDst" -ForegroundColor Cyan
Invoke-Adb @("shell", "ls", "-la", $externalDst)

# Copy into internal app files when run-as is available (debug builds).
$copyOk = $true
$tmp = "/data/local/tmp/deepseek-skills-bundle"
try {
    Invoke-Adb @("shell", "rm", "-rf", $tmp)
    Invoke-Adb @("shell", "mkdir", "-p", $tmp)
    Invoke-Adb @("push", "$skillsSrc/.", "$tmp/")
    Invoke-Adb @("shell", "run-as", $Package, "rm", "-rf", "files/deepseek-mobile/skills")
    Invoke-Adb @("shell", "run-as", $Package, "mkdir", "-p", "files/deepseek-mobile/skills")
    Invoke-Adb @(
        "shell",
        "run-as",
        $Package,
        "cp",
        "-R",
        "$tmp/.",
        "files/deepseek-mobile/skills/"
    )
    Invoke-Adb @(
        "shell",
        "run-as",
        $Package,
        "sh",
        "-c",
        "find files/deepseek-mobile/skills -maxdepth 2 -name SKILL.md -print"
    )
} catch {
    $copyOk = $false
    Write-Host "run-as copy skipped (app may not be debuggable): $_" -ForegroundColor Yellow
}

if ($copyOk) {
    Write-Host "Skills copied to internal files/deepseek-mobile/skills" -ForegroundColor Green
} else {
    Write-Host "Use external skills path only; reopen Skills tab in app." -ForegroundColor Yellow
}
