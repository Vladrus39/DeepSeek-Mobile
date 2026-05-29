# Full pipeline: cargo test -> dx build android -> adb install -> launch -> quick-action smoke.
# Outputs everything under target\verify\. Writes target\verify\PROGRESS.txt as it goes
# and target\verify\STATUS.txt at the end (OK or FAILED ...).

$ErrorActionPreference = "Continue"
$root = $PSScriptRoot
$out  = Join-Path $root "target\verify"
New-Item -ItemType Directory -Force -Path $out | Out-Null
Set-Location -LiteralPath $root

function Now { (Get-Date).ToString("yyyy-MM-dd HH:mm:ss") }
function Progress($msg) {
    "[$([System.Environment]::NewLine.Length)$(Now)] $msg" | Out-Null
    "$(Now)  $msg" | Add-Content -Path (Join-Path $out "PROGRESS.txt")
    Write-Host "$(Now)  $msg" -ForegroundColor Cyan
}

# 0. Environment — set SDK / NDK paths directly (avoid env.ps1's exit-1 path on dot-source).
$localSdk = Join-Path $root "tools\android\sdk"
$env:ANDROID_SDK_ROOT = $localSdk
$env:ANDROID_HOME     = $localSdk
$env:DEEPSEEK_ANDROID_SDK = $localSdk
$ndkDir = Join-Path $localSdk "ndk\26.1.10909125"
if (Test-Path $ndkDir) {
    $env:ANDROID_NDK_HOME = $ndkDir
    $env:NDK_HOME = $ndkDir
}
$env:PATH = (Join-Path $localSdk "platform-tools") + ";" + $env:PATH
$adb = Join-Path $localSdk "platform-tools\adb.exe"
$serial = "RFCNC0PWD4E"

# Open PROGRESS.txt early so failures past this point are visible.
"=== START $(Now) ===" | Out-File -Encoding utf8 -FilePath (Join-Path $out "PROGRESS.txt")
"adb=$adb" | Add-Content (Join-Path $out "PROGRESS.txt")
"cwd=$(Get-Location)" | Add-Content (Join-Path $out "PROGRESS.txt")

# 1. cargo test --workspace
Progress "cargo test --workspace ..."
& cargo "+stable-x86_64-pc-windows-msvc" test --workspace *>&1 |
    Tee-Object -Encoding utf8 -FilePath (Join-Path $out "01-cargo-test.log")
$code = $LASTEXITCODE
Progress "cargo test exit=$code"
if ($code -ne 0) {
    "FAILED cargo test (exit $code)" | Out-File -Path (Join-Path $out "STATUS.txt")
    exit 1
}

# 2. dx build android
Progress "dx build --android --device $serial ..."
& dx build --android --package deepseek-mobile --device $serial --verbose *>&1 |
    Tee-Object -Encoding utf8 -FilePath (Join-Path $out "02-dx-build.log")
$code = $LASTEXITCODE
Progress "dx build exit=$code"
if ($code -ne 0) {
    "FAILED dx build (exit $code)" | Out-File -Path (Join-Path $out "STATUS.txt")
    exit 1
}

# 3. install APK
$apk = Join-Path $root "target\dx\deepseek-mobile\debug\android\app\app\build\outputs\apk\debug\app-debug.apk"
if (-not (Test-Path $apk)) {
    "FAILED apk not found at $apk" | Out-File -Path (Join-Path $out "STATUS.txt")
    exit 1
}
Progress "adb install -r $apk ..."
& $adb -s $serial install -r $apk *>&1 |
    Tee-Object -Encoding utf8 -FilePath (Join-Path $out "03-install.log")
$code = $LASTEXITCODE
Progress "adb install exit=$code"
if ($code -ne 0) {
    "FAILED adb install (exit $code)" | Out-File -Path (Join-Path $out "STATUS.txt")
    exit 1
}

# 4. force-stop + launch
Progress "launch app ..."
& $adb -s $serial shell am force-stop com.deepseek.mobile | Out-Null
Start-Sleep -Seconds 2
& $adb -s $serial shell am start -n com.deepseek.mobile/dev.dioxus.main.MainActivity *>&1 |
    Tee-Object -Encoding utf8 -FilePath (Join-Path $out "04-launch.log")
Start-Sleep -Seconds 8

# 5. screenshot launch state
function Screencap($name) {
    $remote = "/data/local/tmp/verify-$([guid]::NewGuid().ToString('N')).png"
    & $adb -s $serial shell screencap -p $remote | Out-Null
    & $adb -s $serial pull $remote (Join-Path $out $name) | Out-Null
    & $adb -s $serial shell rm -f $remote | Out-Null
}
Screencap "05-launched.png"

# 6. quick-action smoke: ⚡ -> Termux pwd template -> send
Progress "smoke: ⚡ + Termux pwd + send ..."
& $adb -s $serial logcat -c | Out-Null
& $adb -s $serial shell input tap 187 2055 | Out-Null
Start-Sleep -Milliseconds 900
Screencap "06-after-spark.png"
& $adb -s $serial shell input tap 370 1912 | Out-Null
Start-Sleep -Milliseconds 900
Screencap "07-after-template.png"
& $adb -s $serial shell input tap 990 2055 | Out-Null
Progress "waiting 55s for model + tool round-trip ..."
Start-Sleep -Seconds 55
Screencap "08-after-send.png"
& $adb -s $serial logcat -d -v time | Out-File -Encoding utf8 -FilePath (Join-Path $out "09-smoke-logcat.txt")

"OK $(Now)" | Out-File -Path (Join-Path $out "STATUS.txt")
Progress "ALL DONE"
