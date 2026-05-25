param(
    [string]$BinaryPath = "",
    [string]$TaskName = "DeepSeekPcHost"
)

$Root = Split-Path -Parent $PSScriptRoot
if (-not $BinaryPath) {
    $BinaryPath = Join-Path $Root "target\release\deepseek-pc-host.exe"
}

if (-not (Test-Path $BinaryPath)) {
    Write-Error "Build the host first: cargo build -p deepseek-pc-host --release"
}

$Action = New-ScheduledTaskAction -Execute $BinaryPath
$Trigger = New-ScheduledTaskTrigger -AtLogOn
$Settings = New-ScheduledTaskSettingsSet -AllowStartIfOnBatteries -DontStopIfGoingOnBatteries
Register-ScheduledTask -TaskName $TaskName -Action $Action -Trigger $Trigger -Settings $Settings -Force | Out-Null
Start-ScheduledTask -TaskName $TaskName
Write-Host "Registered and started scheduled task '$TaskName'"
