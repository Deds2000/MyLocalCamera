#requires -Version 5.1
<#
.SYNOPSIS
  Installs the S1 spike: Microsoft VirtualCamera sample media source.
.DESCRIPTION
  - Copies VirtualCameraMediaSource.dll and VirtualCamera_Installer.exe
    into C:\Program Files\LocalCameraSpike (FrameServer runs under another
    user, so the DLL must live in a machine-wide path).
  - Registers the DLL (regsvr32 /s). Requires administrator.
  - Prints the exact command to create the virtual camera afterwards.
.NOTES
  Run from an ELEVATED (administrator) PowerShell window:
      powershell -ExecutionPolicy Bypass -File .\install.ps1
  To undo everything: .\uninstall.ps1
#>
[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'

function Test-Admin {
    $id   = [Security.Principal.WindowsIdentity]::GetCurrent()
    $pr  = New-Object Security.Principal.WindowsPrincipal($id)
    return $pr.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
}

if (-not (Test-Admin)) {
    Write-Host ''
    Write-Host '[ERROR] This script requires administrator rights.' -ForegroundColor Red
    Write-Host 'How to run elevated:' -ForegroundColor Red
    Write-Host '  1. Start Menu -> type "PowerShell" -> right-click -> "Run as administrator"'
    Write-Host '  2. cd D:\Repositorios\MyLocalCamera\spikes\s1'
    Write-Host '  3. powershell -ExecutionPolicy Bypass -File .\install.ps1'
    Write-Host ''
    exit 1
}

$BuildOut = Join-Path $PSScriptRoot '_vendor\Windows-Camera\Samples\VirtualCamera\x64\Release'
$SrcDll   = Join-Path $BuildOut 'VirtualCameraMediaSource.dll'
$SrcExe   = Join-Path $BuildOut 'VirtualCamera_Installer.exe'

$DestDir  = 'C:\Program Files\LocalCameraSpike'
$DestDll  = Join-Path $DestDir 'VirtualCameraMediaSource.dll'
$DestExe  = Join-Path $DestDir 'VirtualCamera_Installer.exe'

foreach ($f in @($SrcDll, $SrcExe)) {
    if (-not (Test-Path -LiteralPath $f)) {
        Write-Host "[ERROR] Build artifact not found: $f" -ForegroundColor Red
        Write-Host 'Build the sample first (commands in spikes/s1/NOTAS.md).' -ForegroundColor Red
        exit 1
    }
}

Write-Host "Copying files to $DestDir ..."
New-Item -ItemType Directory -Force -Path $DestDir | Out-Null
Copy-Item -LiteralPath $SrcDll -Destination $DestDll -Force
Copy-Item -LiteralPath $SrcExe -Destination $DestExe -Force
Write-Host "  $DestDll"
Write-Host "  $DestExe"

Write-Host 'Registering media source DLL (regsvr32 /s) ...'
& regsvr32 /s $DestDll
if ($LASTEXITCODE -ne 0) {
    Write-Host "[ERROR] regsvr32 failed (exit code $LASTEXITCODE) for $DestDll" -ForegroundColor Red
    exit 1
}
Write-Host 'Registered CLSID {7B89B92E-FE71-42D0-8A41-E137D06EA184} (VirtualCameraMediaSource).' -ForegroundColor Green

Write-Host ''
Write-Host '=================================================================' -ForegroundColor Green
Write-Host ' NEXT STEP - create the virtual camera (keep this elevated):'   -ForegroundColor Green
Write-Host ''
Write-Host '   & "C:\Program Files\LocalCameraSpike\VirtualCamera_Installer.exe"'
Write-Host ''
Write-Host ' Then answer the prompts:'
Write-Host '   1  -> register'
Write-Host '   1  -> VCam-SimpleMediaSource (synthetic test pattern)'
Write-Host '   1  -> lifetime: System (stays until removed or reboot)'
Write-Host '   1  -> access: Current User'
Write-Host '   5  -> quit (after the camera is registered)'
Write-Host ''
Write-Host ' This creates the virtual camera named "SWVCamMediaSource".'
Write-Host ' Check it in: Settings -> Bluetooth & devices -> Cameras.'
Write-Host ''
Write-Host ' To remove everything later: .\uninstall.ps1'
Write-Host '=================================================================' -ForegroundColor Green
