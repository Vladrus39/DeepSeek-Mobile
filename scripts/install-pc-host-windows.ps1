param(
    [string]$BinaryPath = "",
    [string]$TaskName = "DeepSeekPcHost",
    [string]$EnvFile = ""
)

$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent $PSScriptRoot
if (-not $BinaryPath) {
    $Bundled = Join-Path $Root "tools\pc-host\bin\windows-x86_64\deepseek-pc-host.exe"
    if (Test-Path $Bundled) {
        $BinaryPath = $Bundled
    } else {
        $BinaryPath = Join-Path $Root "target\release\deepseek-pc-host.exe"
    }
}

if (-not (Test-Path $BinaryPath)) {
    Write-Error "Build the host first: cargo build -p deepseek-pc-host --release (or run scripts/build-pc-host-bundles.ps1)"
}

if ($EnvFile -and (Test-Path $EnvFile)) {
    Get-Content $EnvFile | ForEach-Object {
        if ($_ -match '^\s*([^#=]+)=(.*)$') {
            Set-Item -Path "Env:$($matches[1].Trim())" -Value $matches[2].Trim()
        }
    }
}

$Action = New-ScheduledTaskAction -Execute $BinaryPath
$Trigger = New-ScheduledTaskTrigger -AtLogOn
$Settings = New-ScheduledTaskSettingsSet `
    -AllowStartIfOnBatteries `
    -DontStopIfGoingOnBatteries `
    -RestartCount 3 `
    -RestartInterval (New-TimeSpan -Minutes 1)
Register-ScheduledTask -TaskName $TaskName -Action $Action -Trigger $Trigger -Settings $Settings -Force | Out-Null
Start-ScheduledTask -TaskName $TaskName
Write-Host "Registered and started scheduled task '$TaskName' -> $BinaryPath"
