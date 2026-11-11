#requires -version 5.1
<#
soak-f2.ps1 — F2-T15: coletor de evidência dos testes de longa duração
(LT-2.01: 30 min transmitindo; LT-2.02 reaproveita o mesmo formato).

SÓ LÊ: não inicia, não para e não mata nenhum processo (regra de ouro —
especialmente o ffmpeg do usuário).

Amostra a cada 1 s durante o stream e escreve CSV (cultura invariável —
lição do LT-1.01):

  t      segundos desde o início
  fps    último video_fps do log do servidor (vazio até a 1ª linha "stats")
  kbps   último video_kbps do log do servidor
  mem_mb PSS do app (dumpsys meminfo; vazio se o app não estiver rodando)
  temp_c temperatura (thermalservice; fallback /sys; vazio se indisponível)

Pré-requisito — servidor de teste com o log gravado na mesma pasta:
  cargo run -q -p lc-server --example soak_server_lan -- --fresh 2>&1 | Tee-Object soak-server.log

Uso (raiz do repositório):
  .\scripts\soak-f2.ps1                                    # 30 min, 1 s
  .\scripts\soak-f2.ps1 -DurationSec 600 -Out soak-pilot.csv
#>
param(
    [string]$DeviceSerial = "",  # vazio = aparelho único conectado
    [int]$DurationSec = 1800,
    [string]$Out = "soak-f2.csv",
    [string]$ServerLog = "soak-server.log",
    [string]$Package = "com.localcamera.app",
    [int]$IntervalMs = 1000
)

$ErrorActionPreference = "Stop"
$inv = [System.Globalization.CultureInfo]::InvariantCulture
$adbArgs = if ($DeviceSerial) { @("-s", $DeviceSerial) } else { @() }

"t,fps,kbps,mem_mb,temp_c" | Out-File -FilePath $Out -Encoding ascii

# Lê só o novo trecho do log do servidor (não reprocessa o arquivo todo).
$lastOffset = 0
$lastFps = ""
$lastKbps = ""

# ADB pode perder o transporte USB durante a coleta. Nesse caso, ainda assim
# grava a amostra (com campos do aparelho vazios) e tenta novamente no próximo
# intervalo, em vez de encerrar o soak por um erro transitório do adb.exe.
function Invoke-AdbCapture {
    param([string[]]$Arguments)
    try {
        $output = & adb @adbArgs @Arguments 2>$null
        if ($LASTEXITCODE -ne 0) { return @() }
        return $output
    } catch {
        return @()
    }
}

function Read-ServerStats {
    param([int]$Offset)
    if (-not (Test-Path $ServerLog)) { return }
    $size = (Get-Item $ServerLog).Length
    if ($size -lt $Offset) { $Offset = 0 } # log rotated/truncated
    if ($size -le $Offset) { return }
    $stream = [System.IO.File]::Open(
        $ServerLog,
        [System.IO.FileMode]::Open,
        [System.IO.FileAccess]::Read,
        [System.IO.FileShare]::ReadWrite
    )
    try {
        $stream.Position = $Offset
        $reader = [System.IO.StreamReader]::new($stream, [System.Text.Encoding]::UTF8, $true, 1024, $true)
        try { $chunk = $reader.ReadToEnd() } finally { $reader.Dispose() }
    } finally { $stream.Dispose() }
    $script:lastOffset = $size
    $m = [regex]::Matches($chunk, "stats session=\S+ fps=([\d.]+) kbps=([\d.]+)")
    if ($m.Count -gt 0) {
        $hit = $m[$m.Count - 1]
        $script:lastFps = $hit.Groups[1].Value
        $script:lastKbps = $hit.Groups[2].Value
    }
}

# Memória PSS (KB → MB) de `TOTAL` do dumpsys meminfo.
function Read-MemMb {
    $mi = Invoke-AdbCapture -Arguments @("shell", "dumpsys", "meminfo", $Package)
    $line = $mi | Where-Object { $_ -match "(?i)^\s*TOTAL\s+\d+" } | Select-Object -First 1
    if (-not $line) { return "" }
    $m = [regex]::Match($line, "(?i)^\s*TOTAL\s+(\d+)")
    if (-not $m.Success) { return "" }
    return [math]::Round([int]$m.Groups[1].Value / 1024, 1).ToString($inv)
}

# Temperatura em °C: thermalservice (°C direto ou milicelsius) → /sys.
function Read-TempC {
    $th = Invoke-AdbCapture -Arguments @("shell", "dumpsys", "thermalservice")
    $line = $th | Where-Object { $_ -match "(?i)temperature" } | Select-Object -First 1
    if ($line) {
        $num = [regex]::Matches($line, "-?\d+") | ForEach-Object { [int]$_.Value } |
            Where-Object { $_ -ge 1000 -and $_ -le 99999 } | Select-Object -First 1
        if ($null -ne $num) { return [math]::Round($num / 1000, 1).ToString($inv) }
    }
    $tz = Invoke-AdbCapture -Arguments @("shell", "cat", "/sys/class/thermal/thermal_zone0/temp")
    if ($tz -match "^\s*(-?\d+)\s*$") {
        $v = [int]$Matches[1]
        if ($v -ge 1000 -and $v -le 99999) { return [math]::Round($v / 1000, 1).ToString($inv) }
    }
    return ""
}

$sw = [System.Diagnostics.Stopwatch]::StartNew()
$first = [DateTimeOffset]::Now.ToUnixTimeSeconds()
$nextSampleAt = 0
Write-Host ("soak-f2: {0}s → {1} (início utc {2})" -f $DurationSec, $Out, $first)

while ($sw.ElapsedMilliseconds -lt ($DurationSec * 1000)) {
    $t = [int]$sw.Elapsed.TotalSeconds
    Read-ServerStats -Offset $lastOffset
    $fps = $lastFps
    $kbps = $lastKbps
    $mem = Read-MemMb
    $temp = Read-TempC
    "{0},{1},{2},{3},{4}" -f $t, $fps, $kbps, $mem, $temp |
        Out-File -FilePath $Out -Append -Encoding ascii
    if ($t % 30 -eq 0 -and $t -gt 0) {
        Write-Host ("  t={0}s fps={1} kbps={2} mem={3}MB temp={4}C" -f $t, $fps, $kbps, $mem, $temp)
    }
    # Cadência fixa: desconta o tempo gasto nas chamadas ADB para aproximar
    # a frequência real do IntervalMs, em vez de somar 1 s de sleep por ciclo.
    $nextSampleAt += $IntervalMs
    $sleepMs = $nextSampleAt - $sw.ElapsedMilliseconds
    if ($sleepMs -gt 0) {
        Start-Sleep -Milliseconds $sleepMs
    } else {
        $nextSampleAt = $sw.ElapsedMilliseconds
    }
}

$lines = (Get-Content $Out).Count
Write-Host ("soak-f2: fim — {0} linhas em {1}" -f $lines, $Out)
