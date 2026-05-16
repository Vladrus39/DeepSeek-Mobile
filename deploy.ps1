param(
    [Parameter(Position = 0)]
    [string]$Message = ""
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

$repoRoot = Split-Path -Parent $MyInvocation.MyCommand.Path

function Invoke-Git {
    param(
        [Parameter(ValueFromRemainingArguments = $true)]
        [string[]]$Arguments
    )

    $output = & git @Arguments
    if ($LASTEXITCODE -ne 0) {
        throw "git $($Arguments -join ' ') failed with exit code $LASTEXITCODE."
    }

    return $output
}

Push-Location $repoRoot
try {
    if (-not (Test-Path ".git")) {
        throw "This script must be run from a Git repository."
    }

    $branch = (Invoke-Git rev-parse --abbrev-ref HEAD | Select-Object -First 1).Trim()
    if ([string]::IsNullOrWhiteSpace($branch) -or $branch -eq "HEAD") {
        throw "Cannot deploy from a detached HEAD state."
    }

    Invoke-Git add -A | Out-Null
    $pendingChanges = Invoke-Git status --porcelain

    if (-not [string]::IsNullOrWhiteSpace($pendingChanges)) {
        if ([string]::IsNullOrWhiteSpace($Message)) {
            $Message = "sync: $(Get-Date -Format 'yyyy-MM-dd HH:mm:ss')"
        }

        Invoke-Git commit -m $Message | Out-Host
    }
    else {
        Write-Host "No local changes to commit."
    }

    Invoke-Git pull --rebase origin $branch | Out-Host
    Invoke-Git push origin $branch | Out-Host

    Write-Host ""
    Write-Host "Deployed branch '$branch' to origin."
    Invoke-Git status --short --branch | Out-Host
}
finally {
    Pop-Location
}
