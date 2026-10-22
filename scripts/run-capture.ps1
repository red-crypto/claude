# Flash the app and capture the defmt/RTT log for N seconds (default 10).
# Exit 0 if no panic/ERROR appeared in the log, 1 otherwise.
# Usage: pwsh scripts/run-capture.ps1 [-Seconds 10] [-Release]
param([int]$Seconds = 10, [switch]$Release)
$Fw = Resolve-Path (Join-Path $PSScriptRoot '../firmware')
$LogDir = Join-Path $Fw '../logs'; New-Item -ItemType Directory -Force $LogDir | Out-Null
$Log = Join-Path $LogDir ("run-" + (Get-Date -Format 'yyyyMMdd-HHmmss') + ".log")
$extra = @(); if ($Release) { $extra = @('--release') }

Push-Location $Fw
cargo build @extra
if ($LASTEXITCODE -ne 0) { Pop-Location; exit 2 }
$p = Start-Process -FilePath 'cmd.exe' -ArgumentList '/c', "cargo run $($extra -join ' ') > `"$Log`" 2>&1" `
    -NoNewWindow -PassThru
$null = $p.Handle  # keep handle so ExitCode/WaitForExit work on PS 5.1
if (-not $p.WaitForExit($Seconds * 1000)) { & taskkill /T /F /PID $p.Id | Out-Null }
Pop-Location

Get-Content $Log
Write-Host "LOG: $Log"
if (Select-String -Path $Log -Pattern 'panicked|ERROR|HardFault' -Quiet) { Write-Host 'RESULT: FAIL'; exit 1 }
Write-Host 'RESULT: PASS'
