param(
    [string]$RepoUrl = "https://github.com/Vladrus39/DeepSeek-Mobile.git",
    [string]$RepoDir = (Join-Path $HOME "DeepSeek-Mobile"),
    [string]$DeepSeekApiKey = "",
    [switch]$SkipTests,
    [switch]$BuildPcHostBundle,
    [switch]$StartPcHost,
    [string]$PcHostBind = "127.0.0.1:8787",
    [string]$PcHostToken = "",
    [switch]$OpenFirewall,
    [switch]$EnableMdnsFirewall,
    [switch]$InstallAutostart
)

$ErrorActionPreference = "Stop"

function Write-Step {
    param([string]$Message)
    Write-Host ""
    Write-Host "==> $Message" -ForegroundColor Cyan
}

function Write-Ok {
    param([string]$Message)
    Write-Host "[OK] $Message" -ForegroundColor Green
}

function Write-Warn {
    param([string]$Message)
    Write-Host "[WARN] $Message" -ForegroundColor Yellow
}

function Test-Command {
    param([string]$Name)
    return $null -ne (Get-Command $Name -ErrorAction SilentlyContinue)
}

function Refresh-Path {
    $machine = [Environment]::GetEnvironmentVariable("Path", "Machine")
    $user = [Environment]::GetEnvironmentVariable("Path", "User")
    $cargoBin = Join-Path $HOME ".cargo\bin"
    $env:Path = "$machine;$user;$cargoBin"
}

function Require-Winget {
    if (Test-Command winget) {
        return
    }
    throw "winget was not found. Install App Installer from Microsoft Store, reopen PowerShell, then run this script again."
}

function Install-WingetPackage {
    param(
        [string]$Id,
        [string]$Name,
        [string]$Override = ""
    )

    Require-Winget
    Write-Step "Installing $Name via winget if missing"

    $args = @("install", "--id", $Id, "-e", "--accept-source-agreements", "--accept-package-agreements")
    if ($Override) {
        $args += @("--override", $Override)
    }

    & winget @args
    if ($LASTEXITCODE -ne 0) {
        throw "winget failed while installing $Name ($Id). Exit code: $LASTEXITCODE"
    }
    Refresh-Path
}

function Ensure-Git {
    if (Test-Command git) {
        Write-Ok "Git is available: $((git --version) -join ' ')"
        return
    }
    Install-WingetPackage -Id "Git.Git" -Name "Git"
    if (-not (Test-Command git)) {
        throw "Git was installed but is still not available in PATH. Reopen PowerShell and rerun this script."
    }
    Write-Ok "Git is available: $((git --version) -join ' ')"
}

function Ensure-Rust {
    if (-not (Test-Command rustup)) {
        Install-WingetPackage -Id "Rustlang.Rustup" -Name "Rustup"
    }
    if (-not (Test-Command rustup)) {
        throw "rustup was installed but is still not available in PATH. Reopen PowerShell and rerun this script."
    }

    Write-Step "Installing/updating stable MSVC Rust toolchain"
    rustup toolchain install stable-x86_64-pc-windows-msvc
    if ($LASTEXITCODE -ne 0) { throw "rustup toolchain install failed" }

    rustup default stable-x86_64-pc-windows-msvc
    if ($LASTEXITCODE -ne 0) { throw "rustup default failed" }

    Refresh-Path
    if (-not (Test-Command cargo)) {
        throw "cargo is not available after Rust setup. Reopen PowerShell and rerun this script."
    }
    Write-Ok "Rust is available: $((rustc --version) -join ' ')"
    Write-Ok "Cargo is available: $((cargo --version) -join ' ')"
}

function Test-MsvcBuildTools {
    $vswhere = Join-Path ${env:ProgramFiles(x86)} "Microsoft Visual Studio\Installer\vswhere.exe"
    if (-not (Test-Path $vswhere)) {
        return $false
    }

    $installPath = & $vswhere -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath 2>$null
    return -not [string]::IsNullOrWhiteSpace($installPath)
}

function Ensure-MsvcBuildTools {
    if (Test-MsvcBuildTools) {
        Write-Ok "MSVC Build Tools are available"
        return
    }

    Install-WingetPackage `
        -Id "Microsoft.VisualStudio.2022.BuildTools" `
        -Name "Visual Studio 2022 Build Tools / MSVC" `
        -Override "--wait --passive --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended --norestart"

    if (-not (Test-MsvcBuildTools)) {
        Write-Warn "MSVC Build Tools were requested, but detection still failed. Reboot or reopen PowerShell if cargo build fails."
    } else {
        Write-Ok "MSVC Build Tools are available"
    }
}

function Ensure-WebView2 {
    $runtimeCandidates = @(
        Join-Path ${env:ProgramFiles(x86)} "Microsoft\EdgeWebView\Application",
        Join-Path $env:ProgramFiles "Microsoft\EdgeWebView\Application"
    )
    foreach ($candidate in $runtimeCandidates) {
        if (Test-Path $candidate) {
            Write-Ok "WebView2 Runtime appears to be available"
            return
        }
    }

    try {
        Install-WingetPackage -Id "Microsoft.EdgeWebView2Runtime" -Name "Microsoft Edge WebView2 Runtime"
    } catch {
        Write-Warn "Could not auto-install WebView2 Runtime via winget. Desktop UI may fail until WebView2 Runtime is installed. Details: $($_.Exception.Message)"
        return
    }
    Write-Ok "WebView2 Runtime setup completed"
}

function Sync-Repository {
    Write-Step "Preparing repository at $RepoDir"

    if (Test-Path $RepoDir) {
        if (-not (Test-Path (Join-Path $RepoDir ".git"))) {
            throw "$RepoDir already exists but is not a Git repository. Move it or pass -RepoDir to another location."
        }
        git -C $RepoDir fetch --all --prune
        if ($LASTEXITCODE -ne 0) { throw "git fetch failed" }
        git -C $RepoDir checkout main
        if ($LASTEXITCODE -ne 0) { throw "git checkout main failed" }
        git -C $RepoDir pull --ff-only
        if ($LASTEXITCODE -ne 0) { throw "git pull --ff-only failed. Resolve local changes or clone to another -RepoDir." }
    } else {
        git clone $RepoUrl $RepoDir
        if ($LASTEXITCODE -ne 0) { throw "git clone failed" }
    }

    Set-Location $RepoDir
    Write-Ok "Repository is ready: $RepoDir"
}

function Ensure-EnvFile {
    $envFile = Join-Path $RepoDir ".env"
    if (Test-Path $envFile) {
        Write-Ok ".env already exists; not overwriting it"
        return
    }

    if ($DeepSeekApiKey) {
        @"
DEEPSEEK_API_KEY=$DeepSeekApiKey
GITHUB_TOKEN=
DEEPSEEK_MOBILE_DATA_DIR=.deepseek-mobile
"@ | Set-Content -Encoding UTF8 $envFile
        Write-Ok ".env created with provided DeepSeek API key"
    } else {
        @"
# Fill this key before real agent/chat use.
DEEPSEEK_API_KEY=sk-your-key-here

# Optional
# GITHUB_TOKEN=ghp_...
# DEEPSEEK_MOBILE_DATA_DIR=.deepseek-mobile
"@ | Set-Content -Encoding UTF8 $envFile
        Write-Warn ".env created with placeholder DEEPSEEK_API_KEY. Build/check can continue, but real API calls need a valid key."
    }
}

function Invoke-CargoChecks {
    Write-Step "Running Rust workspace check"
    cargo +stable-x86_64-pc-windows-msvc check --workspace --all-targets
    if ($LASTEXITCODE -ne 0) { throw "cargo check failed" }
    Write-Ok "cargo check passed"

    if (-not $SkipTests) {
        Write-Step "Running Rust workspace tests"
        cargo +stable-x86_64-pc-windows-msvc test --workspace
        if ($LASTEXITCODE -ne 0) { throw "cargo test failed" }
        Write-Ok "cargo test passed"
    } else {
        Write-Warn "Tests skipped by -SkipTests"
    }
}

function Build-PcHostBundle {
    Write-Step "Building PC Host release binary and copying it to tools/pc-host/bin"
    & (Join-Path $RepoDir "scripts\build-pc-host-bundles.ps1")
    if ($LASTEXITCODE -ne 0) { throw "PC Host bundle build failed" }
    Write-Ok "PC Host bundle is ready"
}

function Open-PcHostFirewall {
    Write-Step "Opening Windows Firewall port for PC Host LAN access"
    $port = ($PcHostBind -split ":")[-1]
    if (-not ($port -match '^\d+$')) {
        throw "Cannot parse port from PcHostBind: $PcHostBind"
    }

    $ruleName = "DeepSeek PC Host $port"
    $existing = Get-NetFirewallRule -DisplayName $ruleName -ErrorAction SilentlyContinue
    if (-not $existing) {
        New-NetFirewallRule -DisplayName $ruleName -Direction Inbound -Action Allow -Protocol TCP -LocalPort ([int]$port) | Out-Null
        Write-Ok "Firewall rule created: $ruleName"
    } else {
        Write-Ok "Firewall rule already exists: $ruleName"
    }
}

function Install-PcHostAutostart {
    Write-Step "Installing PC Host autostart task"
    $envFile = Join-Path $RepoDir ".pc-host.env"
    @"
DEEPSEEK_PC_HOST_BIND=$PcHostBind
DEEPSEEK_PC_HOST_WORKSPACE=$RepoDir
DEEPSEEK_PC_HOST_TOKEN=$PcHostToken
DEEPSEEK_PC_HOST_LABEL=Developer PC
DEEPSEEK_PC_HOST_ID=pc-local
DEEPSEEK_PC_HOST_WORKSPACE_ID=local
"@ | Set-Content -Encoding UTF8 $envFile

    & (Join-Path $RepoDir "scripts\install-pc-host-windows.ps1") -EnvFile $envFile
    if ($LASTEXITCODE -ne 0) { throw "PC Host autostart installation failed" }
    Write-Ok "PC Host autostart task installed"
}

function Start-PcHostForeground {
    Write-Step "Starting PC Host in the current PowerShell window"
    $env:DEEPSEEK_PC_HOST_BIND = $PcHostBind
    $env:DEEPSEEK_PC_HOST_WORKSPACE = $RepoDir
    $env:DEEPSEEK_PC_HOST_TOKEN = $PcHostToken
    $env:DEEPSEEK_PC_HOST_LABEL = "Developer PC"
    $env:DEEPSEEK_PC_HOST_ID = "pc-local"
    $env:DEEPSEEK_PC_HOST_WORKSPACE_ID = "local"

    Write-Host "PC Host URL: http://$PcHostBind"
    if ($PcHostToken) {
        Write-Host "Token: $PcHostToken"
    } else {
        Write-Warn "No token was set. Use -PcHostToken for real LAN use."
    }
    cargo run -p deepseek-pc-host
}

Write-Step "DeepSeek-Mobile Windows PC bootstrap"
Ensure-Git
Ensure-Rust
Ensure-MsvcBuildTools
Ensure-WebView2
Sync-Repository
Ensure-EnvFile
Invoke-CargoChecks

if ($BuildPcHostBundle -or $InstallAutostart) {
    Build-PcHostBundle
}

if ($OpenFirewall) {
    Open-PcHostFirewall
}

if ($EnableMdnsFirewall) {
    Write-Step "Opening Windows Firewall for PC Host LAN + mDNS"
    & (Join-Path $RepoDir "scripts\enable-pc-host-mdns-windows.ps1") -TcpPort ([int](($PcHostBind -split ":")[-1]))
    if ($LASTEXITCODE -ne 0) { throw "enable-pc-host-mdns-windows.ps1 failed" }
    Write-Ok "mDNS/firewall rules applied"
}

if ($InstallAutostart) {
    Install-PcHostAutostart
}

Write-Host ""
Write-Ok "PC setup completed"
Write-Host "Repository: $RepoDir"
Write-Host "Desktop UI: cargo run -p deepseek-mobile"
Write-Host "PC Host:    cargo run -p deepseek-pc-host"

if ($StartPcHost) {
    Start-PcHostForeground
}
