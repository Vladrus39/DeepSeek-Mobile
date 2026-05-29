# Enable allow-external-apps in Termux by typing the commands into its terminal over ADB.
# Run from PowerShell. Space is encoded as %s for `input text`; the whole typed string is
# single-quoted so the DEVICE shell passes `>>` and `~` literally to `input text`.

$ErrorActionPreference = "Continue"
$adb = Join-Path $PSScriptRoot "tools\android\sdk\platform-tools\adb.exe"

function Type-Line($encoded) {
    & $adb shell "input text '$encoded'" | Out-Null
    Start-Sleep -Milliseconds 400
    & $adb shell input keyevent 66 | Out-Null   # ENTER
    Start-Sleep -Milliseconds 900
}

function Shot($name) {
    $remote = "/data/local/tmp/$name"
    & $adb shell screencap -p $remote | Out-Null
    & $adb pull $remote (Join-Path $PSScriptRoot "target\$name") | Out-Null
    & $adb shell rm -f $remote | Out-Null
}

# Make sure Termux is foreground.
& $adb shell am start -n com.termux/.app.TermuxActivity | Out-Null
Start-Sleep -Seconds 2

Type-Line "mkdir%s-p%s~/.termux%s~/deepseek-project"
Type-Line "echo%sallow-external-apps=true%s>>%s~/.termux/termux.properties"
Type-Line "termux-reload-settings"
Start-Sleep -Milliseconds 800
Shot "termux-setup.png"

# Verify the file contents.
Type-Line "cat%s~/.termux/termux.properties"
Start-Sleep -Milliseconds 700
Shot "termux-verify.png"

Write-Host "DONE"
