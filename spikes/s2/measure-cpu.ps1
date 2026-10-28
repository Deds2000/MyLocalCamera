#requires -Version 5.1
<#
.SYNOPSIS
  Spike S2 — mede CPU por 60 s nos dois lados do canal:
    - o "app" (s2-pipe-server ou s2-shm-server, roda como usuário comum);
    - o Frame Server (svchost.exe -k Camera, sessão 0), onde a DLL roda.

.DESCRIPTION
  Compara TotalProcessorTime antes/depois do intervalo. O valor é % de UM
  núcleo (pode passar de 100% se o processo usar várias threads).
  Pré-requisito: a câmera virtual está ABERTA em um app consumidor (ex.:
  app Câmera/OBS) para o Frame Server estar rodando.

.EXAMPLE
  .\measure-cpu.ps1                 # 60 s, grava em results\s2-cpu-60s.json
  .\measure-cpu.ps1 -Seconds 30 -Out results\outro.json
#>
[CmdletBinding()]
param(
    [int]$Seconds = 60,
    [string]$Out = (Join-Path $PSScriptRoot 'results\s2-cpu-60s.json')
)
$ErrorActionPreference = 'Stop'

# 1) "app": processo do servidor do canal.
$procNames = @('s2-pipe-server', 's2-shm-server')
$server = $null
foreach ($n in $procNames) {
    $p = Get-Process -Name $n -ErrorAction SilentlyContinue | Select-Object -First 1
    if ($p) { $server = $p; break }
}
if (-not $server) {
    Write-Host '[ERRO] Nenhum servidor do canal em execução (s2-pipe-server / s2-shm-server).' -ForegroundColor Red
    exit 1
}

# 2) Frame Server: svchost.exe -k Camera (serviço FrameServer, sessão 0).
$fs = Get-CimInstance Win32_Process -Filter "Name = 'svchost.exe'" |
    Where-Object { $_.CommandLine -like '*-k Camera*' } |
    Select-Object -First 1
if (-not $fs) {
    Write-Host '[ERRO] Frame Server não está rodando. Abra a câmera virtual em um app consumidor primeiro.' -ForegroundColor Red
    exit 1
}

Write-Host "Medindo CPU por $Seconds s:"
Write-Host "  app  : $($server.ProcessName) (PID $($server.Id))"
Write-Host "  frame: svchost.exe -k Camera (PID $($fs.ProcessId))"
Write-Host 'Aguarde ...'

function Get-CpuMillis([int]$procId) {
    $p = Get-Process -Id $procId -ErrorAction SilentlyContinue
    if (-not $p) { return $null }
    return [double]$p.TotalProcessorTime.TotalMilliseconds
}

function To-Percent([double]$before, [double]$after) {
    if ($null -eq $before -or $null -eq $after -or $after -lt $before) {
        return $null # processo terminou ou reiniciou durante a medição
    }
    return [math]::Round(($after - $before) / ($Seconds * 10.0), 2)
}

$s1 = Get-CpuMillis $server.Id
$f1 = Get-CpuMillis ([int]$fs.ProcessId)
Start-Sleep -Seconds $Seconds
$s2 = Get-CpuMillis $server.Id
$f2 = Get-CpuMillis ([int]$fs.ProcessId)

$serverCpu = To-Percent $s1 $s2
$fsCpu     = To-Percent $f1 $f2

$result = [ordered]@{
    seconds   = $Seconds
    server    = [ordered]@{ name = $server.ProcessName; pid = $server.Id; cpu_percent_of_core = $serverCpu }
    frameserver = [ordered]@{ name = 'svchost.exe -k Camera'; pid = [int]$fs.ProcessId; cpu_percent_of_core = $fsCpu }
    note      = 'cpu_percent_of_core = % de UM núcleo (pode > 100% com múltiplas threads); null = processo terminou/reiniciou durante a medição'
}

$dir = Split-Path $Out -Parent
if ($dir -and -not (Test-Path $dir)) { New-Item -ItemType Directory -Force -Path $dir | Out-Null }
$result | ConvertTo-Json | Set-Content -LiteralPath $Out -Encoding UTF8

Write-Host ''
Write-Host ('CPU {0,5} s | server: {1,8} % | FrameServer: {2,8} %  (de 1 núcleo)' -f $Seconds, $serverCpu, $fsCpu)
Write-Host "JSON: $Out"
