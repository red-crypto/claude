# Full pipeline: fmt -> clippy -> host unit tests -> firmware build -> HIL tests.
# Usage: pwsh scripts/check.ps1 [-Hil]   (-Hil: board required, fail if no probe)
# Logs: logs/check-<timestamp>/<stage>.log ; last line prints a summary.
param([switch]$Hil, [int]$HilTimeoutSec = 300)
$ErrorActionPreference = 'Continue'
$Root = Resolve-Path (Join-Path $PSScriptRoot '..')
$LogDir = Join-Path $Root ("logs/check-" + (Get-Date -Format 'yyyyMMdd-HHmmss'))
New-Item -ItemType Directory -Force $LogDir | Out-Null
$Results = @(); $Failed = $false

function Invoke-Stage([string]$Name, [string]$Dir, [string]$Cmd, [int]$TimeoutSec = 0) {
    Write-Host "== STAGE: $Name =="
    $log = Join-Path $LogDir "$Name.log"
    $p = Start-Process -FilePath 'cmd.exe' -ArgumentList '/c', "$Cmd > `"$log`" 2>&1" `
        -WorkingDirectory (Join-Path $Root $Dir) -NoNewWindow -PassThru
    $null = $p.Handle  # keep handle so ExitCode is available on PS 5.1
    if ($TimeoutSec -gt 0) {
        if (-not $p.WaitForExit($TimeoutSec * 1000)) {
            & taskkill /T /F /PID $p.Id | Out-Null
            Add-Content $log "TIMEOUT after $TimeoutSec s"
        }
    }
    $p.WaitForExit()
    if ($p.ExitCode -eq 0) { $script:Results += "$Name=PASS" }
    else {
        $script:Results += "$Name=FAIL"; $script:Failed = $true
        Get-Content $log -Tail 40
    }
}

Invoke-Stage 'fmt'         'logic'    'cargo fmt --check'
Invoke-Stage 'fmt-fw'      'firmware' 'cargo fmt --check'
Invoke-Stage 'clippy'      'logic'    'cargo clippy --all-targets -- -D warnings'
Invoke-Stage 'clippy-fw'   'firmware' 'cargo clippy -- -D warnings'
Invoke-Stage 'host-test'   'logic'    'cargo test'
Invoke-Stage 'build'       'firmware' 'cargo build'
Invoke-Stage 'build-tests' 'firmware' 'cargo test --no-run'

$probe = $null
if (Get-Command probe-rs -ErrorAction SilentlyContinue) {
    $probe = (& probe-rs list 2>$null) | Select-String -Pattern 'st-?link|cmsis|j-?link'
}
if ($probe) { Invoke-Stage 'hil' 'firmware' 'cargo test' $HilTimeoutSec }
elseif ($Hil) { Write-Host '== STAGE: hil == no debug probe found'; $Results += 'hil=FAIL(no-probe)'; $Failed = $true }
else { $Results += 'hil=SKIP(no-probe)' }

Write-Host "SUMMARY: $($Results -join ' ')  logs=$LogDir"
if ($Failed) { exit 1 } else { exit 0 }
