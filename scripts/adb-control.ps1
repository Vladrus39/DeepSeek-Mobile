# DeepSeek Mobile ADB control center.
#
# Goals:
# - one safe entrypoint for seeing and controlling the connected phone;
# - collect reproducible evidence: screenshots, UI XML, logcat, app files, summary;
# - avoid destructive actions unless an explicit action requests them.
#
# Examples:
#   . .\tools\android\env.ps1
#   .\scripts\adb-control.ps1 -Action Full -Serial RFCNC0PWD4E
#   .\scripts\adb-control.ps1 -Action InstallLaunch -Serial RFCNC0PWD4E
#   .\scripts\adb-control.ps1 -Action Calibrate -Serial RFCNC0PWD4E
#   .\scripts\adb-control.ps1 -Action Tap -X 70 -Y 145
#   .\scripts\adb-control.ps1 -Action Text -Text "Reply with PONG"

param(
    [ValidateSet(
        "Full",
        "Report",
        "InstallLaunch",
        "Launch",
        "Stop",
        "Restart",
        "Capture",
        "Tabs",
        "Calibrate",
        "GrantTermux",
        "Network",
        "Tap",
        "Text",
        "ChatSend",
        "Swipe",
        "Key",
        "Logcat",
        "ClearData",
        "Shell"
    )]
    [string]$Action = "Full",

    [string]$Serial = "",
    [string]$Package = "com.deepseek.mobile",
    [string]$Activity = "dev.dioxus.main.MainActivity",
    [string]$Apk = "",
    [string]$OutDir = "",

    # Generic input parameters for Tap/Swipe/Key/Text.
    [int]$X = -1,
    [int]$Y = -1,
    [int]$X2 = -1,
    [int]$Y2 = -1,
    [int]$DurationMs = 300,
    [string]$Text = "",
    [int]$KeyCode = 4,

    # Wait after launch/taps. -1 chooses a sensible default per action.
    [int]$WaitSeconds = -1,

    # Optional behaviour toggles.
    [switch]$StayAwake,
    [switch]$ClearLogcat,
    [switch]$NoScreens,
    [switch]$OpenTermux,

    # Remaining args are passed to `adb shell` when -Action Shell is used.
    [Parameter(ValueFromRemainingArguments = $true)]
    [string[]]$ShellArgs
)

$ErrorActionPreference = "Stop"

$ProjectRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$DefaultApk = Join-Path $ProjectRoot "target\dx\deepseek-mobile\debug\android\app\app\build\outputs\apk\debug\app-debug.apk"
if (-not $Apk) { $Apk = $DefaultApk }
if (-not $OutDir) {
    $stamp = Get-Date -Format "yyyyMMdd-HHmmss"
    $OutDir = Join-Path $ProjectRoot "target\adb-control\$stamp"
}
New-Item -ItemType Directory -Force -Path $OutDir | Out-Null

function Resolve-AdbPath {
    $local = Join-Path $ProjectRoot "tools\android\sdk\platform-tools\adb.exe"
    if (Test-Path $local) { return $local }

    if ($env:ANDROID_SDK_ROOT) {
        $sdkAdb = Join-Path $env:ANDROID_SDK_ROOT "platform-tools\adb.exe"
        if (Test-Path $sdkAdb) { return $sdkAdb }
    }

    if ($env:ANDROID_HOME) {
        $homeAdb = Join-Path $env:ANDROID_HOME "platform-tools\adb.exe"
        if (Test-Path $homeAdb) { return $homeAdb }
    }

    return "adb"
}

$Adb = Resolve-AdbPath

function Invoke-Adb {
    param(
        [Parameter(Mandatory = $true)]
        [string[]]$AdbArgs,
        [switch]$AllowFail
    )

    $prefix = @()
    if ($Serial) { $prefix = @("-s", $Serial) }
    $oldEap = $ErrorActionPreference
    $ErrorActionPreference = "Continue"
    try {
        $output = & $Adb @prefix @AdbArgs 2>&1
        $code = $LASTEXITCODE
    } finally {
        $ErrorActionPreference = $oldEap
    }
    if ($code -ne 0 -and -not $AllowFail) {
        throw "adb $($AdbArgs -join ' ') failed with code ${code}: $($output | Out-String)"
    }
    return $output
}

function Save-Text {
    param([string]$Name, [object]$Content)
    $path = Join-Path $OutDir $Name
    ($Content | Out-String).TrimEnd() | Set-Content -Path $path -Encoding UTF8
    return $path
}

function Select-Device {
    if ($Serial) { return }

    $devices = & $Adb devices -l 2>&1
    $rows = @($devices | Where-Object { $_ -match "\sdevice\s" })
    if ($rows.Count -eq 0) {
        Save-Text "adb-devices.txt" $devices | Out-Null
        throw "No ADB device is connected. Saved adb-devices.txt in $OutDir"
    }
    if ($rows.Count -gt 1) {
        Save-Text "adb-devices.txt" $devices | Out-Null
        throw "Multiple ADB devices are connected. Pass -Serial. Saved adb-devices.txt in $OutDir"
    }
    $script:Serial = ($rows[0] -split "\s+")[0]
}

function Wait-Default {
    param([string]$ForAction)
    if ($WaitSeconds -ge 0) { return $WaitSeconds }
    switch ($ForAction) {
        "Calibrate" { return 90 }
        "InstallLaunch" { return 7 }
        "Restart" { return 7 }
        "Launch" { return 6 }
        "Tabs" { return 2 }
        default { return 3 }
    }
}

function Get-ScreenSize {
    $wm = Invoke-Adb @("shell", "wm", "size") -AllowFail
    $line = ($wm | Select-String -Pattern "Physical size:\s*(\d+)x(\d+)" | Select-Object -First 1)
    if ($line -and $line.Matches.Count -gt 0) {
        return @{
            Width = [int]$line.Matches[0].Groups[1].Value
            Height = [int]$line.Matches[0].Groups[2].Value
        }
    }
    return @{ Width = 1080; Height = 2400 }
}

function Save-Screenshot {
    param([string]$Name = "screen.png")
    if ($NoScreens) { return $null }
    $path = Join-Path $OutDir $Name
    $remote = "/data/local/tmp/deepseek-mobile-screen-$([guid]::NewGuid().ToString("N")).png"
    Invoke-Adb @("shell", "screencap", "-p", $remote) | Out-Null
    Invoke-Adb @("pull", $remote, $path) | Out-Null
    Invoke-Adb @("shell", "rm", "-f", $remote) -AllowFail | Out-Null

    $bytes = [System.IO.File]::ReadAllBytes($path)
    if ($bytes.Length -lt 8 -or $bytes[0] -ne 0x89 -or $bytes[1] -ne 0x50 -or $bytes[2] -ne 0x4E -or $bytes[3] -ne 0x47) {
        throw "Screenshot is not a valid PNG: $path"
    }
    return $path
}

function Save-UiDump {
    param([string]$Name = "ui.xml")
    $path = Join-Path $OutDir $Name
    $dump = Invoke-Adb @("exec-out", "uiautomator", "dump", "/dev/tty") -AllowFail
    $dump | Set-Content -Path $path -Encoding UTF8
    return $path
}

function Invoke-RunAs {
    param(
        [Parameter(Mandatory = $true)]
        [string[]]$RunArgs,
        [switch]$AllowFail
    )
    return Invoke-Adb -AdbArgs (@("shell", "run-as", $Package) + $RunArgs) -AllowFail:$AllowFail
}

function Get-RunAsText {
    param([string[]]$RunArgs)
    try {
        return Invoke-RunAs $RunArgs -AllowFail
    } catch {
        return $_.Exception.Message
    }
}

function Save-AppFileIfExists {
    param([string]$DevicePath, [string]$Name)
    $content = Get-RunAsText @("cat", $DevicePath)
    if (($content | Out-String) -match "No such file|Permission denied|not debuggable") {
        return $null
    }
    return Save-Text $Name $content
}

function Grant-TermuxPermission {
    Invoke-Adb @("shell", "pm", "grant", $Package, "com.termux.permission.RUN_COMMAND") -AllowFail | Out-Null
}

function Ensure-AppWorkspace {
    Invoke-RunAs @("mkdir", "-p", "files/deepseek-mobile/workspace") -AllowFail | Out-Null
    Invoke-RunAs @("mkdir", "-p", "files/deepseek-mobile/skills") -AllowFail | Out-Null

    $existingReadme = Get-RunAsText @("ls", "files/deepseek-mobile/workspace/README.md")
    if (($existingReadme | Out-String) -notmatch "No such file|No such|cannot access") {
        return
    }

    $readme = Join-Path $env:TEMP "deepseek-mobile-workspace-README.md"
    Set-Content -Path $readme -Encoding UTF8 -Value @"
# DeepSeek Mobile workspace

Import a ZIP from Files, use PC Host, or clone a repo in Termux.
"@
    Invoke-Adb @("push", $readme, "/data/local/tmp/deepseek-mobile-workspace-README.md") -AllowFail | Out-Null
    Invoke-RunAs @("cp", "/data/local/tmp/deepseek-mobile-workspace-README.md", "files/deepseek-mobile/workspace/README.md") -AllowFail | Out-Null
}

function Start-App {
    param([switch]$ForceStop)
    if ($ForceStop) {
        Invoke-Adb @("shell", "am", "force-stop", $Package) -AllowFail | Out-Null
        Start-Sleep -Milliseconds 700
    }
    Invoke-Adb @("shell", "am", "start", "-n", "$Package/$Activity") | Out-Null
    Start-Sleep -Seconds (Wait-Default $Action)
}

function Stop-App {
    Invoke-Adb @("shell", "am", "force-stop", $Package) -AllowFail | Out-Null
}

function Convert-AdbInputText {
    param([string]$Value)
    # `adb shell input text` uses %s for spaces. Keep this intentionally simple:
    # for complex text, prefer clipboard/manual paste.
    return ($Value -replace "%", "%25" -replace " ", "%s" -replace "'", "\'")
}

function Tap-Phone {
    param([int]$TapX, [int]$TapY)
    Invoke-Adb @("shell", "input", "tap", "$TapX", "$TapY") | Out-Null
}

function Collect-Report {
    param([string]$Prefix = "report")

    $summary = [ordered]@{}
    $summary["timestamp"] = (Get-Date).ToString("s")
    $summary["project_root"] = $ProjectRoot
    $summary["adb"] = $Adb
    $summary["serial"] = $Serial
    $summary["package"] = $Package

    $summary["devices_file"] = Save-Text "$Prefix-adb-devices.txt" (& $Adb devices -l 2>&1)
    $summary["model"] = (Invoke-Adb @("shell", "getprop", "ro.product.model") -AllowFail | Out-String).Trim()
    $summary["android"] = (Invoke-Adb @("shell", "getprop", "ro.build.version.release") -AllowFail | Out-String).Trim()
    $summary["sdk"] = (Invoke-Adb @("shell", "getprop", "ro.build.version.sdk") -AllowFail | Out-String).Trim()
    $summary["wm_size"] = (Invoke-Adb @("shell", "wm", "size") -AllowFail | Out-String).Trim()
    $summary["wm_density"] = (Invoke-Adb @("shell", "wm", "density") -AllowFail | Out-String).Trim()
    $summary["airplane_mode"] = (Invoke-Adb @("shell", "settings", "get", "global", "airplane_mode_on") -AllowFail | Out-String).Trim()
    $summary["battery_file"] = Save-Text "$Prefix-battery.txt" (Invoke-Adb @("shell", "dumpsys", "battery") -AllowFail)
    $summary["focus_file"] = Save-Text "$Prefix-window-focus.txt" (Invoke-Adb @("shell", "dumpsys", "window") -AllowFail | Select-String -Pattern "mCurrentFocus|mFocusedApp|mDreamingLockscreen|mShowingDream")
    $summary["package_file"] = Save-Text "$Prefix-package.txt" (Invoke-Adb @("shell", "dumpsys", "package", $Package) -AllowFail)
    $summary["termux_package"] = (Invoke-Adb @("shell", "pm", "list", "packages", "com.termux") -AllowFail | Out-String).Trim()
    $summary["pid"] = (Invoke-Adb @("shell", "pidof", "-s", $Package) -AllowFail | Out-String).Trim()

    $runAsProbe = Get-RunAsText @("pwd")
    $summary["run_as"] = if (($runAsProbe | Out-String) -match "run-as:|not debuggable|Package.*unknown") { "FAIL" } else { "OK" }
    $summary["app_files_file"] = Save-Text "$Prefix-app-files.txt" (Get-RunAsText @("find", "files/deepseek-mobile", "-maxdepth", "4", "-type", "f", "-print"))
    $summary["app_root_ls_file"] = Save-Text "$Prefix-app-root-ls.txt" (Get-RunAsText @("ls", "-la", "files/deepseek-mobile"))

    Save-AppFileIfExists "files/deepseek-mobile/config.json" "$Prefix-config.json" | Out-Null
    Save-AppFileIfExists "files/deepseek-mobile/termux_workspace.json" "$Prefix-termux-workspace.json" | Out-Null
    Save-AppFileIfExists "files/deepseek-mobile/chat_sessions.json" "$Prefix-chat-sessions.json" | Out-Null
    Save-AppFileIfExists "files/deepseek-mobile/workspace_connections.json" "$Prefix-workspace-connections.json" | Out-Null
    Save-AppFileIfExists "files/deepseek-mobile/mcp.json" "$Prefix-mcp.json" | Out-Null
    Save-AppFileIfExists "files/deepseek-mobile/.calibration_trace" "$Prefix-calibration-trace.txt" | Out-Null
    Save-AppFileIfExists "files/deepseek-mobile/.agent_calibrated_v1" "$Prefix-agent-calibrated.txt" | Out-Null

    $summary["screenshot"] = Save-Screenshot "$Prefix-screen.png"
    $summary["ui_dump"] = Save-UiDump "$Prefix-ui.xml"

    $logPath = Join-Path $OutDir "$Prefix-logcat.txt"
    Invoke-Adb @("logcat", "-d", "-v", "time", "-t", "3000") -AllowFail | Set-Content -Path $logPath -Encoding UTF8
    $summary["logcat"] = $logPath

    $logText = Get-Content $logPath -ErrorAction SilentlyContinue
    $summary["fatal_hits"] = @($logText | Select-String -Pattern "FATAL EXCEPTION").Count
    $summary["android_runtime_lines"] = @($logText | Select-String -Pattern "AndroidRuntime").Count
    $summary["anr_hits"] = @($logText | Select-String -Pattern "Input dispatching timed out|ANR in |Application Not Responding: $([regex]::Escape($Package))").Count
    $summary["run_command_hits"] = @($logText | Select-String -Pattern "RUN_COMMAND|RunCommandService|DeepSeekTermuxBridge").Count

    $md = @()
    $md += "# DeepSeek Mobile ADB report"
    $md += ""
    foreach ($key in $summary.Keys) {
        $md += "- **$key**: $($summary[$key])"
    }
    Save-Text "$Prefix-summary.md" ($md -join "`n") | Out-Null

    return $summary
}

function Collect-NetworkDiagnostics {
    $summary = [ordered]@{}
    $summary["timestamp"] = (Get-Date).ToString("s")
    $summary["serial"] = $Serial
    $summary["airplane_mode"] = (Invoke-Adb @("shell", "settings", "get", "global", "airplane_mode_on") -AllowFail | Out-String).Trim()
    $summary["wifi_file"] = Save-Text "network-wifi.txt" (
        Invoke-Adb @("shell", "dumpsys", "wifi") -AllowFail |
            Select-String -Pattern "Wi-Fi is|mWifiInfo SSID|WifiStatus:|IP:|Supplicant state|isValidated" -CaseSensitive:$false
    )
    $summary["connectivity_file"] = Save-Text "network-connectivity.txt" (
        Invoke-Adb @("shell", "dumpsys", "connectivity") -AllowFail |
            Select-String -Pattern "NetworkAgentInfo|VPN|Firewall|OwnerUid|tun0|VALIDATED|DnsAddresses|UnderlyingNetworks" -CaseSensitive:$false |
            Select-Object -First 160
    )
    foreach ($targetHost in @("api.deepseek.com", "deepseek.com", "google.com", "github.com")) {
        $name = "network-ping-$($targetHost -replace '[^a-zA-Z0-9]+','-').txt"
        $summary["ping_$targetHost"] = Save-Text $name (Invoke-Adb @("shell", "ping", "-c", "1", $targetHost) -AllowFail)
    }

    $md = @("# DeepSeek Mobile ADB network report", "")
    foreach ($key in $summary.Keys) {
        $md += "- **$key**: $($summary[$key])"
    }
    Save-Text "network-summary.md" ($md -join "`n") | Out-Null
    return $summary
}

function Run-TabsSmoke {
    $size = Get-ScreenSize
    $w = $size.Width
    $h = $size.Height
    $y = [int]($h * 0.938)
    $tabs = @(
        @{ Name = "chat"; X = [int]($w * 0.105) },
        @{ Name = "skills"; X = [int]($w * 0.265) },
        @{ Name = "mcp"; X = [int]($w * 0.425) },
        @{ Name = "pc"; X = [int]($w * 0.575) },
        @{ Name = "files"; X = [int]($w * 0.735) },
        @{ Name = "git"; X = [int]($w * 0.895) }
    )

    if ($ClearLogcat) { Invoke-Adb @("logcat", "-c") -AllowFail | Out-Null }

    foreach ($tab in $tabs) {
        Tap-Phone $tab.X $y
        Start-Sleep -Seconds (Wait-Default "Tabs")
        Save-Screenshot ("tab-{0}.png" -f $tab.Name) | Out-Null
    }
    Save-UiDump "tabs-ui.xml" | Out-Null
    Invoke-Adb @("logcat", "-d", "-v", "time", "-t", "2000") -AllowFail | Set-Content -Path (Join-Path $OutDir "tabs-logcat.txt") -Encoding UTF8
}

function Install-LatestApk {
    if (-not (Test-Path $Apk)) {
        throw "APK not found: $Apk. Build first: . .\tools\android\env.ps1; dx build --android --package deepseek-mobile --device $Serial"
    }
    Invoke-Adb @("install", "-r", $Apk) | Out-Host
}

function Request-Calibration {
    Grant-TermuxPermission
    Ensure-AppWorkspace
    Invoke-RunAs @("rm", "-f", "files/deepseek-mobile/.agent_calibrated_v1") -AllowFail | Out-Null
    Invoke-RunAs @("touch", "files/deepseek-mobile/.agent_calibration_requested_v1") -AllowFail | Out-Null
    if ($OpenTermux) {
        Invoke-Adb @("shell", "am", "start", "-n", "com.termux/.app.TermuxActivity") -AllowFail | Out-Null
        Write-Host "If Termux is not configured, run once inside Termux:" -ForegroundColor Yellow
        Write-Host "  mkdir -p ~/.termux ~/deepseek-project"
        Write-Host "  echo allow-external-apps=true >> ~/.termux/termux.properties"
        Write-Host "  termux-reload-settings"
        Write-Host "Then re-run: .\scripts\adb-control.ps1 -Action Calibrate -Serial $Serial"
        return
    }
    Start-App -ForceStop
    $wait = Wait-Default "Calibrate"
    Write-Host "Waiting ${wait}s for app-side calibration callback..." -ForegroundColor Yellow
    Start-Sleep -Seconds $wait
    $cal = Get-RunAsText @("cat", "files/deepseek-mobile/.agent_calibrated_v1")
    Save-Text "calibration-result.txt" $cal | Out-Null
    $trace = Get-RunAsText @("cat", "files/deepseek-mobile/.calibration_trace")
    Save-Text "calibration-trace.txt" $trace | Out-Null
    if (($cal | Out-String) -match "ok") {
        Write-Host "Calibration PASS" -ForegroundColor Green
    } else {
        Write-Host "Calibration not completed. See calibration-trace.txt in $OutDir" -ForegroundColor Yellow
    }
}

Select-Device

Write-Host "ADB: $Adb" -ForegroundColor DarkGray
Write-Host "Device: $Serial" -ForegroundColor DarkGray
Write-Host "Out: $OutDir" -ForegroundColor DarkGray

if ($StayAwake) {
    Invoke-Adb @("shell", "svc", "power", "stayon", "true") -AllowFail | Out-Null
}
if ($ClearLogcat) {
    Invoke-Adb @("logcat", "-c") -AllowFail | Out-Null
}

switch ($Action) {
    "Full" {
        Grant-TermuxPermission
        Ensure-AppWorkspace
        Start-App -ForceStop
        Collect-Report "01-launch" | Out-Null
        Run-TabsSmoke
        Collect-Report "02-after-tabs" | Out-Null
        Write-Host "Full report completed: $OutDir" -ForegroundColor Green
    }
    "Report" {
        Collect-Report "report" | Out-Null
        Write-Host "Report completed: $OutDir" -ForegroundColor Green
    }
    "InstallLaunch" {
        Install-LatestApk
        Grant-TermuxPermission
        Ensure-AppWorkspace
        Start-App -ForceStop
        Collect-Report "install-launch" | Out-Null
        Write-Host "Install + launch completed: $OutDir" -ForegroundColor Green
    }
    "Launch" {
        Start-App
        Save-Screenshot "launch.png" | Out-Null
        Write-Host "Launch completed: $OutDir" -ForegroundColor Green
    }
    "Stop" {
        Stop-App
        Write-Host "App stopped." -ForegroundColor Green
    }
    "Restart" {
        Start-App -ForceStop
        Save-Screenshot "restart.png" | Out-Null
        Write-Host "Restart completed: $OutDir" -ForegroundColor Green
    }
    "Capture" {
        Save-Screenshot "capture.png" | Out-Null
        Save-UiDump "capture-ui.xml" | Out-Null
        Write-Host "Capture completed: $OutDir" -ForegroundColor Green
    }
    "Tabs" {
        Run-TabsSmoke
        Write-Host "Tabs smoke completed: $OutDir" -ForegroundColor Green
    }
    "Calibrate" {
        Request-Calibration
    }
    "GrantTermux" {
        Grant-TermuxPermission
        $pkgDump = Invoke-Adb @("shell", "dumpsys", "package", $Package) -AllowFail
        Save-Text "grant-termux-package.txt" $pkgDump | Out-Null
        Write-Host "Termux permission grant attempted. See grant-termux-package.txt" -ForegroundColor Green
    }
    "Network" {
        Collect-NetworkDiagnostics | Out-Null
        Write-Host "Network diagnostics completed: $OutDir" -ForegroundColor Green
    }
    "Tap" {
        if ($X -lt 0 -or $Y -lt 0) { throw "-Action Tap requires -X and -Y" }
        Tap-Phone $X $Y
        Start-Sleep -Seconds (Wait-Default "Tap")
        Save-Screenshot "after-tap.png" | Out-Null
        Write-Host "Tap completed: $OutDir" -ForegroundColor Green
    }
    "Text" {
        if (-not $Text) { throw "-Action Text requires -Text" }
        Invoke-Adb @("shell", "input", "text", (Convert-AdbInputText $Text)) | Out-Null
        Start-Sleep -Seconds (Wait-Default "Text")
        Save-Screenshot "after-text.png" | Out-Null
        Write-Host "Text input completed: $OutDir" -ForegroundColor Green
    }
    "ChatSend" {
        # Full chat round-trip: focus input, type prompt, close keyboard, tap send,
        # wait for the model, then dump screenshots + logcat + persisted chat history.
        # Coordinates default to the cockpit chat bar; override with -X/-Y (input) and -X2/-Y2 (send).
        if (-not $Text) { throw "-Action ChatSend requires -Text" }
        Invoke-Adb @("shell", "am", "start", "-n", "$Package/$Activity") -AllowFail | Out-Null
        Start-Sleep -Seconds 2
        $size = Get-ScreenSize
        $inX  = if ($X  -ge 0) { $X  } else { [int]($size.Width  * 0.42) }
        $inY  = if ($Y  -ge 0) { $Y  } else { [int]($size.Height * 0.857) }
        $sndX = if ($X2 -ge 0) { $X2 } else { [int]($size.Width  * 0.92) }
        $sndY = if ($Y2 -ge 0) { $Y2 } else { [int]($size.Height * 0.857) }

        if ($ClearLogcat) { Invoke-Adb @("logcat", "-c") -AllowFail | Out-Null }

        Tap-Phone $inX $inY
        Start-Sleep -Milliseconds 700
        Invoke-Adb @("shell", "input", "text", (Convert-AdbInputText $Text)) | Out-Null
        Start-Sleep -Milliseconds 500
        Save-Screenshot "chatsend-1-typed.png" | Out-Null
        Invoke-Adb @("shell", "input", "keyevent", "4") -AllowFail | Out-Null  # close keyboard
        Start-Sleep -Milliseconds 600
        Save-Screenshot "chatsend-2-before-send.png" | Out-Null

        Tap-Phone $sndX $sndY
        $wait = if ($WaitSeconds -ge 0) { $WaitSeconds } else { 45 }
        Write-Host "Sent. Waiting ${wait}s for model/tool response..." -ForegroundColor Yellow
        Start-Sleep -Seconds $wait
        Save-Screenshot "chatsend-3-after.png" | Out-Null

        # Persisted chat + config (ground-truth text of the assistant turn / error).
        Save-AppFileIfExists "files/deepseek-mobile/chat_sessions.json" "chatsend-chat-sessions.json" | Out-Null
        Save-AppFileIfExists "files/deepseek-mobile/config.json" "chatsend-config.json" | Out-Null
        Save-Text "chatsend-app-files.txt" (Get-RunAsText @("find", "files/deepseek-mobile", "-maxdepth", "5", "-type", "f", "-print"))
        Save-Text "chatsend-runtime-sessions.txt" (Get-RunAsText @("find", "files/deepseek-mobile/runtime_store/sessions", "-type", "f", "-exec", "cat", "{}", ";"))

        $logPath = Join-Path $OutDir "chatsend-logcat.txt"
        Invoke-Adb @("logcat", "-d", "-v", "time", "-t", "6000") -AllowFail | Set-Content -Path $logPath -Encoding UTF8
        Write-Host "ChatSend completed: $OutDir" -ForegroundColor Green
    }
    "Swipe" {
        if ($X -lt 0 -or $Y -lt 0 -or $X2 -lt 0 -or $Y2 -lt 0) { throw "-Action Swipe requires -X -Y -X2 -Y2" }
        Invoke-Adb @("shell", "input", "swipe", "$X", "$Y", "$X2", "$Y2", "$DurationMs") | Out-Null
        Start-Sleep -Seconds (Wait-Default "Swipe")
        Save-Screenshot "after-swipe.png" | Out-Null
        Write-Host "Swipe completed: $OutDir" -ForegroundColor Green
    }
    "Key" {
        Invoke-Adb @("shell", "input", "keyevent", "$KeyCode") | Out-Null
        Start-Sleep -Seconds (Wait-Default "Key")
        Save-Screenshot "after-key.png" | Out-Null
        Write-Host "Key event completed: $OutDir" -ForegroundColor Green
    }
    "Logcat" {
        Invoke-Adb @("logcat", "-d", "-v", "time", "-t", "5000") -AllowFail | Set-Content -Path (Join-Path $OutDir "logcat.txt") -Encoding UTF8
        Write-Host "Logcat saved: $OutDir" -ForegroundColor Green
    }
    "ClearData" {
        Write-Host "Clearing app data for $Package (destructive by requested action)..." -ForegroundColor Yellow
        Invoke-Adb @("shell", "pm", "clear", $Package) | Out-Host
    }
    "Shell" {
        if (-not $ShellArgs -or $ShellArgs.Count -eq 0) { throw "-Action Shell requires remaining adb shell args, e.g. -- dumpsys window" }
        Invoke-Adb (@("shell") + $ShellArgs) | Out-Host
    }
}
