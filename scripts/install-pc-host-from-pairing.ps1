param(
    [string]$BundleDir = "",
    [string]$TaskName = "DeepSeekPcHost"
)

$ErrorActionPreference = "Stop"
if (-not $BundleDir) {
    $BundleDir = $PSScriptRoot
}

$EnvFile = Join-Path $BundleDir "deepseek-pc-host.env"
if (-not (Test-Path $EnvFile)) {
    Write-Error "Missing deepseek-pc-host.env in $BundleDir. Unzip the pairing bundle first."
}

Get-Content $EnvFile | ForEach-Object {
    if ($_ -match '^\s*([^#=]+)=(.*)$') {
        $name = $matches[1].Trim()
        $value = $matches[2].Trim()
        Set-Item -Path "Env:$name" -Value $value
    }
}

$HostCandidates = @(
    (Join-Path $BundleDir "deepseek-pc-host.exe"),
    (Join-Path $BundleDir "deepseek-pc-host"),
    (Join-Path $BundleDir "bin\deepseek-pc-host.exe")
)
$BinaryPath = $HostCandidates | Where-Object { Test-Path $_ } | Select-Object -First 1
if (-not $BinaryPath) {
    Write-Error "deepseek-pc-host binary not found next to pairing bundle. Run start-deepseek-pc-host.ps1 once or run scripts/build-pc-host-bundles.ps1 on a dev machine."
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
Write-Host "Registered scheduled task '$TaskName' using $BinaryPath"
Write-Host "Workspace: $env:DEEPSEEK_PC_HOST_WORKSPACE | Bind: $env:DEEPSEEK_PC_HOST_BIND"
