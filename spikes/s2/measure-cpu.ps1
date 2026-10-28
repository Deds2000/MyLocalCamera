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
    # Nota: $PSScriptRoot não existe no default de param no Windows PowerShell
    # 5.1 (só no corpo do script) — por isso o default é vazio.
    [string]$Out = ''
)
$ErrorActionPreference = 'Stop'
if (-not $Out) {
    $Out = Join-Path $PSScriptRoot 'results\s2-cpu-60s.json'
}

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
#    Descoberta via `tasklist /svc`: funciona sem elevação (o CommandLine do
#    svchost do SYSTEM não é legível por usuário comum via CIM/WMI).
#    Pode haver MAIS DE UM svchost hospedando o serviço (grupos compartilhados);
#    medimos todos e somamos (o que trabalha é o que tem CPU).
$fsPids = @()
foreach ($line in (tasklist /svc 2>$null)) {
    if ($line -match 'FrameServer' -and $line -match '^\s*svchost\.exe\s+(\d+)') {
        $fsPids += [int]$Matches[1]
    }
}
$fsPids = @($fsPids | Sort-Object -Unique)
if ($fsPids.Count -eq 0) {
    # Fallback: CIM (funciona se o script for executado como administrador).
    $fsCim = Get-CimInstance Win32_Process -Filter "Name = 'svchost.exe'" |
        Where-Object { $_.CommandLine -like '*-k Camera*' } |
        Select-Object -First 1
    if ($fsCim) { $fsPids = @([int]$fsCim.ProcessId) }
}
if ($fsPids.Count -eq 0) {
    Write-Host '[ERRO] Frame Server não encontrado (tasklist /svc sem FrameServer). Abra a câmera virtual em um app consumidor primeiro.' -ForegroundColor Red
    exit 1
}

Write-Host "Medindo CPU por $Seconds s:"
Write-Host "  app  : $($server.ProcessName) (PID $($server.Id))"
Write-Host "  frame: svchost -k Camera (PID $($fsPids -join ', '))"
Write-Host 'Aguarde ...'

function Get-CpuMillis([int]$procId) {
    $p = Get-Process -Id $procId -ErrorAction SilentlyContinue
    if (-not $p) { return $null }
    $t = $p.TotalProcessorTime
    if ($null -eq $t) { return $null }
    return [double]$t.TotalMilliseconds
}

# Usuário comum NÃO lê GetProcessTimes de svchost da sessão 0 (token filtrado):
# .NET devolve TotalProcessorTime nulo. Fallback pelo CIM/WMI
# (KernelModeTime + UserModeTime, em 100 ns), que o provider lê com privilégio
# próprio — confirmado na máquina (delta vivo durante o stream).
function Get-CimCpuMillis([int]$procId) {
    $c = Get-CimInstance Win32_Process -Filter "ProcessId=$procId" -ErrorAction SilentlyContinue
    if (-not $c) { return $null }
    return [double]([long]$c.KernelModeTime + [long]$c.UserModeTime) / 10000.0 # 100ns -> ms
}

function To-Percent($before, $after) {
    if ($null -eq $before -or $null -eq $after -or $after -lt $before) {
        return $null # processo terminou ou reiniciou durante a medição
    }
    return [math]::Round(($after - $before) / ($Seconds * 10.0), 2)
}

$s1 = Get-CpuMillis $server.Id
$fBefore = @{}
foreach ($p in $fsPids) { $fBefore[$p] = Get-CimCpuMillis $p } # sessão 0: CIM
Start-Sleep -Seconds $Seconds
$s2 = Get-CpuMillis $server.Id
$fAfter = @{}
foreach ($p in $fsPids) { $fAfter[$p] = Get-CimCpuMillis $p }

$serverCpu = To-Percent $s1 $s2
$fsCpu = 0.0
$fsMeasured = 0
foreach ($p in $fsPids) {
    $v = To-Percent $fBefore[$p] $fAfter[$p]
    if ($null -ne $v) { $fsCpu += $v; $fsMeasured++ }
}
if ($fsMeasured -eq 0) { $fsCpu = $null }

$result = [ordered]@{
    seconds   = $Seconds
    server    = [ordered]@{ name = $server.ProcessName; pid = $server.Id; cpu_percent_of_core = $serverCpu }
    frameserver = [ordered]@{ name = 'svchost.exe -k Camera'; pids = $fsPids; cpu_percent_of_core = $fsCpu }
    note      = 'cpu_percent_of_core = % de UM núcleo (pode > 100% com múltiplas threads); null = processo terminou/reiniciou durante a medição'
}

$dir = Split-Path $Out -Parent
if ($dir -and -not (Test-Path $dir)) { New-Item -ItemType Directory -Force -Path $dir | Out-Null }
$result | ConvertTo-Json | Set-Content -LiteralPath $Out -Encoding UTF8

Write-Host ''
Write-Host ('CPU {0,5} s | server: {1,8} % | FrameServer: {2,8} %  (de 1 núcleo)' -f $Seconds, $serverCpu, $fsCpu)
Write-Host "JSON: $Out"

# Sanidade: se o FrameServer não gastou CPU, a câmera provavelmente não estava
# aberta durante a medição (a DLL só trabalha quando há cliente de câmera).
if (($null -eq $fsCpu) -or ($fsCpu -lt 0.5)) {
    Write-Host '[AVISO] FrameServer praticamente sem CPU: a câmera virtual provavelmente NÃO estava aberta durante a medição. Reabra a câmera e rode de novo.' -ForegroundColor Yellow
}
