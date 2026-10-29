#Requires -Version 5.1
<#
.SYNOPSIS
  Custo de decodificação H.264 (passo 3 do cartão F0-S5, modo B).

  MÉTODO (velocidade, não contabilidade de CPU): nesta máquina a contabilidade
  de CPU por processo (Process.CPU / GetProcessTimes) mostrou-se inconsistente
  para o ffmpeg (valores decrescentes entre amostras), então o custo é derivado
  da VELOCIDADE de decodificação (frames/s), que é objetiva e reproduzível:

    custo_1_núcleo = fps_preview / fps_dec (30 fps do preview / velocidade medida)

  O fixture hd30.h264 tem 300 frames (12 s @ 25 fps); cada variante decodifica
  o arquivo inteiro em velocidade máxima.

  Variantes:
    sw  : ffmpeg -i hd30.h264 -f null -                  (todos os núcleos)
    sw1 : ffmpeg -threads 1 -i hd30.h264 -f null -       (1 núcleo)
    hw  : ffmpeg -hwaccel d3d11va -i hd30.h264 -f null - (GPU)

  Saída: docs\spikes\tmp\s5-decode.json + linha RESUMO.
  NÃO interfere com outros processos ffmpeg (só espera o PID que lança).
#>
[CmdletBinding()]
param(
    [int]$MinFrames = 250
)
$ErrorActionPreference = 'Stop'
[System.Threading.Thread]::CurrentThread.CurrentCulture = [System.Globalization.CultureInfo]::InvariantCulture
$spikeDir = (Resolve-Path $PSScriptRoot).Path
$repoRoot = Split-Path -Parent (Split-Path -Parent $spikeDir)
$ffmpeg   = 'C:\ProgramData\chocolatey\bin\ffmpeg.exe'
$fixture  = Join-Path $repoRoot 'fixtures\hd30.h264'
$outDir   = Join-Path $repoRoot 'docs\spikes\tmp'
New-Item -ItemType Directory -Force $outDir | Out-Null
$outJson  = Join-Path $outDir 's5-decode.json'
$previewFps = 30.0  # fps do preview (base do custo)

function Measure-Speed([string]$name, [string[]]$ffArgs) {
    $log = Join-Path $env:TEMP "s5-decode-$name.err"
    Remove-Item $log -ErrorAction SilentlyContinue
    $t0 = Get-Date
    $p = Start-Process -FilePath $ffmpeg -ArgumentList (@('-hide_banner') + $ffArgs) `
        -WorkingDirectory $repoRoot -PassThru -WindowStyle Hidden `
        -RedirectStandardError $log
    $p.WaitForExit()
    $wall = [math]::Round(((Get-Date) - $t0).TotalSeconds, 2)
    # (não usar $p.ExitCode: quirk do .NET Framework PS 5.1 — pode ficar nulo.
    #  Sucesso é validado pelos frames decodificados.)
    $frames = 0
    foreach ($line in (Get-Content $log -ErrorAction SilentlyContinue)) {
        if ($line -match 'frame=\s*(\d+)') { $frames = [int]$Matches[1] }
    }
    if ($frames -lt $MinFrames) { throw "ffmpeg ($name) decodificou só $frames (mín. $MinFrames)" }
    if ($wall -lt 0.1) { $wall = 0.1 }
    $fps = [math]::Round($frames / $wall, 1)
    $cost1Core = [math]::Round($previewFps / $fps, 4)  # fração de 1 núcleo para 30 fps
    Write-Host ("  {0,-4} frames={1} wall={2}s speed={3} fps/s custo={4} de 1 núcleo (30 fps)" -f $name, $frames, $wall, $fps, $cost1Core)
    Remove-Item $log -ErrorAction SilentlyContinue
    [pscustomobject]@{ variant = $name; frames = $frames; wall_s = $wall; decode_fps = $fps; cost_fraction_of_1_core_at_30fps = $cost1Core }
}

Write-Host 'mediando velocidade de decodificação (arquivo inteiro: 300 frames)...'
$res = [ordered]@{}
$res.preview_fps = $previewFps
$res.fixture = $fixture
$res.sw  = Measure-Speed 'sw'  @('-loglevel', 'info', '-i', $fixture, '-f', 'null', '-')
$res.sw1 = Measure-Speed 'sw1' @('-loglevel', 'info', '-threads', '1', '-i', $fixture, '-f', 'null', '-')
$res.hw  = Measure-Speed 'hw'  @('-loglevel', 'info', '-hwaccel', 'd3d11va', '-i', $fixture, '-f', 'null', '-')
$res | ConvertTo-Json -Depth 4 | Set-Content -Path $outJson -Encoding ASCII
Write-Host ("RESUMO s5-decode | sw={0}fps sw1={1}fps hw={2}fps | custo30fps sw1={3}% hw~0% | json={4}" -f `
    $res.sw.decode_fps, $res.sw1.decode_fps, $res.hw.decode_fps, `
    [math]::Round($res.sw1.cost_fraction_of_1_core_at_30fps * 100, 1), $outJson)
