#requires -Version 5.1
<#
.SYNOPSIS
  Admin cleanup for S1/S2 spikes - single block for the user to paste (ASCII only, PS 5.1 safe).
.DESCRIPTION
  Order: kill the stuck installer from the elevated worker, stop/unregister the
  LocalCameraElevated scheduled task, run spikes/s1/uninstall.ps1 (virtual camera
  + CLSID + folder), remove HKLM:\SOFTWARE\LocalCameraSpike, remove
  C:\ProgramData\LocalCameraElevated, kill s2-pipe-server (if alive), remove the
  temporary user LocalCamSpikeTest. Prints a final verification of leftovers.
.NOTES
  Run in an ELEVATED (administrator) PowerShell from anywhere:
      powershell -ExecutionPolicy Bypass -File D:\Repositorios\MyLocalCamera\spikes\s2\cleanup-elevado.ps1
#>
[CmdletBinding()]
param()

$ErrorActionPreference = 'Continue'
# spikes/s2/ -> dois niveis acima = raiz do repositorio
$RepoRoot = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)

function Test-Admin {
    $id = [Security.Principal.WindowsIdentity]::GetCurrent()
    return (New-Object Security.Principal.WindowsPrincipal($id)).IsInRole(
        [Security.Principal.WindowsBuiltInRole]::Administrator)
}
if (-not (Test-Admin)) {
    Write-Host '[ERRO] Administrator required. Open PowerShell as Administrator and run again.' -ForegroundColor Red
    exit 1
}

function Step([string]$Msg, [scriptblock]$Block) {
    Write-Host "`n==> $Msg" -ForegroundColor Cyan
    try { & $Block } catch { Write-Host "    [ERRO] $($_.Exception.Message)" -ForegroundColor Yellow }
}

# 1) Stuck installer from the elevated worker (only the spike's, matched by path)
Step 'Kill stuck VirtualCamera_Installer (SYSTEM worker)' {
    Get-Process -Name 'VirtualCamera_Installer' -ErrorAction SilentlyContinue |
        Where-Object { $_.Path -like 'C:\Program Files\LocalCameraSpike\*' -or $_.Path -like 'C:\ProgramData\LocalCameraElevated\*' } |
        ForEach-Object { Write-Host "    kill PID $($_.Id)"; Stop-Process -Id $_.Id -Force }
}

# 2) LocalCameraElevated scheduled task (stop + unregister)
Step 'Scheduled task LocalCameraElevated' {
    Stop-ScheduledTask -TaskName 'LocalCameraElevated' -ErrorAction SilentlyContinue
    Unregister-ScheduledTask -TaskName 'LocalCameraElevated' -Confirm:$false -ErrorAction SilentlyContinue
    if (Get-ScheduledTask -TaskName 'LocalCameraElevated' -ErrorAction SilentlyContinue) {
        Write-Host '    [ERRO] task still exists' -ForegroundColor Yellow
    } else { Write-Host '    removed (or never existed)' -ForegroundColor Green }
}

# 3) S1 uninstall: virtual camera + CLSID + C:\Program Files\LocalCameraSpike
Step 'spikes/s1/uninstall.ps1 (virtual camera + CLSID + folder)' {
    & powershell -ExecutionPolicy Bypass -File (Join-Path $RepoRoot 'spikes\s1\uninstall.ps1')
}

# 4) HKLM:\SOFTWARE\LocalCameraSpike key
Step 'Registry key HKLM:\SOFTWARE\LocalCameraSpike' {
    if (Test-Path 'HKLM:\SOFTWARE\LocalCameraSpike') {
        Remove-Item -LiteralPath 'HKLM:\SOFTWARE\LocalCameraSpike' -Recurse -Force
        Write-Host '    removed' -ForegroundColor Green
    } else { Write-Host '    does not exist (already clean?)' -ForegroundColor Green }
}

# 5) C:\ProgramData\LocalCameraElevated folder
Step 'Folder C:\ProgramData\LocalCameraElevated' {
    if (Test-Path 'C:\ProgramData\LocalCameraElevated') {
        Remove-Item -LiteralPath 'C:\ProgramData\LocalCameraElevated' -Recurse -Force
        Write-Host '    removed' -ForegroundColor Green
    } else { Write-Host '    does not exist (already clean?)' -ForegroundColor Green }
}

# 6) s2-pipe-server process (if still alive)
Step 's2-pipe-server process' {
    Get-Process -Name 's2-pipe-server' -ErrorAction SilentlyContinue |
        ForEach-Object { Write-Host "    kill PID $($_.Id)"; Stop-Process -Id $_.Id -Force }
    if (Get-Process -Name 's2-pipe-server' -ErrorAction SilentlyContinue) {
        Write-Host '    [ERRO] still running' -ForegroundColor Yellow
    } else { Write-Host '    not running' -ForegroundColor Green }
}

# 7) Temporary user LocalCamSpikeTest
Step 'Temporary user LocalCamSpikeTest' {
    net user LocalCamSpikeTest /delete 2>&1 | ForEach-Object { Write-Host "    $_" }
}

# ---------------------------------------------------------------------------
# Final verification
# ---------------------------------------------------------------------------
Write-Host "`n==== FINAL VERIFICATION ====" -ForegroundColor Cyan
$restos = 0
if (Get-ScheduledTask -TaskName 'LocalCameraElevated' -ErrorAction SilentlyContinue) { Write-Host '  [LEFT] task LocalCameraElevated' -ForegroundColor Yellow; $restos++ }
if (Test-Path 'HKLM:\SOFTWARE\LocalCameraSpike') { Write-Host '  [LEFT] HKLM:\SOFTWARE\LocalCameraSpike' -ForegroundColor Yellow; $restos++ }
if (Test-Path 'HKLM:\SOFTWARE\Classes\CLSID\{7B89B92E-FE71-42D0-8A41-E137D06EA184}') { Write-Host '  [LEFT] media source CLSID' -ForegroundColor Yellow; $restos++ }
if (Test-Path 'C:\Program Files\LocalCameraSpike') { Write-Host '  [LEFT] C:\Program Files\LocalCameraSpike' -ForegroundColor Yellow; $restos++ }
if (Test-Path 'C:\ProgramData\LocalCameraElevated') { Write-Host '  [LEFT] C:\ProgramData\LocalCameraElevated' -ForegroundColor Yellow; $restos++ }
if (net user LocalCamSpikeTest 2>$null | Select-String 'LocalCamSpikeTest') { Write-Host '  [LEFT] user LocalCamSpikeTest' -ForegroundColor Yellow; $restos++ }
if (Get-Process -Name 's2-pipe-server','VirtualCamera_Installer' -ErrorAction SilentlyContinue) { Write-Host '  [LEFT] spike process still running' -ForegroundColor Yellow; $restos++ }
if ($restos -eq 0) { Write-Host '  All clean.' -ForegroundColor Green } else { Write-Host "  $restos item(s) left over - see above." -ForegroundColor Yellow }
