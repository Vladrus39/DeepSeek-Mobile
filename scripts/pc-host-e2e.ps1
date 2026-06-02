# PC Host live E2E: starts deepseek-pc-host.exe against a throwaway workspace,
# drives the real gateway HTTP protocol (WriteFile / ExecuteCommand / GitStatus),
# then reads the created files back from disk to prove real file creation.
$ErrorActionPreference = 'Stop'
$root  = Split-Path -Parent $PSScriptRoot
Set-Location $root
$agent = Join-Path $root 'target\agent'
New-Item -ItemType Directory -Force -Path $agent | Out-Null
$out = Join-Path $agent 'pchost-e2e.txt'
$ws  = Join-Path $agent 'pcws'
if (Test-Path $ws) { Remove-Item -Recurse -Force $ws }
New-Item -ItemType Directory -Force -Path $ws | Out-Null

function Log($s) { $s | Out-File -FilePath $out -Append -Encoding utf8 }

"=== PC HOST E2E $(Get-Date -Format 'dd.MM.yyyy HH:mm:ss') ===" | Out-File -FilePath $out -Encoding utf8
Log "workspace=$ws"

# git repo so GitStatus has something real to report
git -C $ws init        2>&1 | Out-Null
git -C $ws config user.email e2e@test.local 2>&1 | Out-Null
git -C $ws config user.name  e2e            2>&1 | Out-Null

$env:DEEPSEEK_PC_HOST_BIND         = '127.0.0.1:8799'
$env:DEEPSEEK_PC_HOST_WORKSPACE    = $ws
$env:DEEPSEEK_PC_HOST_WORKSPACE_ID = 'local'
$env:DEEPSEEK_PC_HOST_POLICY       = 'developer'

Log "--- build deepseek-pc-host --release ---"
& cmd.exe /c "cargo build -p deepseek-pc-host --release >> `"$out`" 2>&1"
if ($LASTEXITCODE -ne 0) { throw "cargo build -p deepseek-pc-host --release failed; see $out" }

$exe = Join-Path $root 'target\release\deepseek-pc-host.exe'
Log "exe=$exe"
$proc = Start-Process -FilePath $exe -PassThru -WindowStyle Hidden `
        -RedirectStandardOutput (Join-Path $agent 'pchost-stdout.txt') `
        -RedirectStandardError  (Join-Path $agent 'pchost-stderr.txt')
Start-Sleep -Seconds 4

$base = 'http://127.0.0.1:8799'
function Req($obj) {
  $body = @{ id=[guid]::NewGuid().ToString(); device_id='e2e'; timestamp_unix=0; request=$obj } |
          ConvertTo-Json -Depth 10 -Compress
  try { Invoke-RestMethod -Uri "$base/v1/gateway/request" -Method Post -ContentType 'application/json' -Body $body |
          ConvertTo-Json -Depth 10 }
  catch { "ERR: $($_.Exception.Message)" }
}

function Health {
        try { Invoke-RestMethod -Uri "$base/health" | ConvertTo-Json -Depth 8 }
        catch { "ERR: $($_.Exception.Message)" }
}

Log "--- health ---"
Log (Health)

Log "--- WriteFile hello.py ---"
Log (Req @{ WriteFile = @{ workspace_id='local'; path='hello.py'; content="print('PC-HOST-OK', sum(range(11)))`n" } })

Log "--- WriteFile pkg/calc.py ---"
Log (Req @{ WriteFile = @{ workspace_id='local'; path='pkg/calc.py'; content="def add(a,b): return a+b`nif __name__=='__main__': print('calc', add(40,2))`n" } })

Log "--- ExecuteCommand: git --version ---"
Log (Req @{ ExecuteCommand = @{ workspace_id='local'; command=@{ program='git'; args=@('--version'); working_dir=$null }; environment_id=$null } })

Log "--- ExecuteCommand: python hello.py ---"
Log (Req @{ ExecuteCommand = @{ workspace_id='local'; command=@{ program='python'; args=@('hello.py'); working_dir=$null }; environment_id=$null } })

Log "--- GitStatus ---"
Log (Req @{ GitStatus = @{ workspace_id='local' } })

Log "--- ReadFile hello.py (back through gateway) ---"
Log (Req @{ ReadFile = @{ workspace_id='local'; path='hello.py' } })

Stop-Process -Id $proc.Id -Force 2>$null

Log "--- DISK: real files on the Windows filesystem ---"
Log (Get-ChildItem -Recurse -File $ws | Select-Object @{n='Rel';e={$_.FullName.Substring($ws.Length+1)}}, Length, LastWriteTime | Format-Table -AutoSize | Out-String)
Log "--- DISK: hello.py contents ---"
Log (Get-Content (Join-Path $ws 'hello.py') -Raw)
Log "--- DISK: pkg/calc.py contents ---"
Log (Get-Content (Join-Path $ws 'pkg\calc.py') -Raw)
if ((Get-Content $out -Raw) -match 'ERR:') { throw "PC Host E2E failed; see $out" }
"done"
