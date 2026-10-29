param(
    [Parameter(Mandatory = $true)][string]$ProcessName,
    [int]$Minutes = 30,
    [string]$Out = "soak.csv",
    [int]$WarmupMinutes = 1
)
$ErrorActionPreference = 'Stop'
# Cultura invariavel para o -f (N1): em pt-BR "8,5" quebra o CSV (viram 2 colunas).
[System.Threading.Thread]::CurrentThread.CurrentCulture = [System.Globalization.CultureInfo]::InvariantCulture

$end = (Get-Date).AddMinutes($Minutes)
"t,cpu_pct,ws_mb,handles,threads" | Set-Content $Out
$prev = $null
$warmupEnd = (Get-Date).AddMinutes($WarmupMinutes)
$first = $null
$last = $null
$maxWs = 0.0
$maxHandles = 0
$maxCpu = 0.0

while ((Get-Date) -lt $end) {
    $p = Get-Process -Name $ProcessName -ErrorAction SilentlyContinue | Select-Object -First 1
    if (-not $p) {
        Write-Warning "processo $ProcessName nao encontrado"
        Start-Sleep 10
        continue
    }
    $now = Get-Date
    if ($prev) {
        $cpu = (($p.TotalProcessorTime - $prev.cpu).TotalSeconds / ($now - $prev.t).TotalSeconds) / [Environment]::ProcessorCount * 100
    } else {
        $cpu = 0
    }
    $prev = @{ cpu = $p.TotalProcessorTime; t = $now }
    $ws = $p.WorkingSet64 / 1MB
    $handles = $p.HandleCount
    $threads = $p.Threads.Count
    "{0:o},{1:N1},{2:N1},{3},{4}" -f $now, $cpu, $ws, $handles, $threads | Add-Content $Out

    if ($now -ge $warmupEnd) {
        if (-not $first) {
            $first = @{ ws = $ws; handles = $handles }
        }
        $last = @{ ws = $ws; handles = $handles }
        if ($ws -gt $maxWs) { $maxWs = $ws }
        if ($handles -gt $maxHandles) { $maxHandles = $handles }
        if ($cpu -gt $maxCpu) { $maxCpu = $cpu }
    }
    Start-Sleep 10
}

if (-not $first) {
    Write-Warning "nenhuma amostra apos o aquecimento ($WarmupMinutes min)"
    exit 1
}

$wsGrowth = $last.ws - $first.ws
$handleGrowth = 0.0
if ($first.handles -gt 0) {
    $handleGrowth = ($last.handles - $first.handles) / $first.handles * 100
}

Write-Host ("resumo do soak ({0} min, aquecimento {1} min)" -f $Minutes, $WarmupMinutes)
Write-Host ("  ws_mb:    inicio {0:N1}  fim {1:N1}  maximo {2:N1}  crescimento {3:N1} MB" -f $first.ws, $last.ws, $maxWs, $wsGrowth)
Write-Host ("  handles:  inicio {0}  fim {1}  maximo {2}  crescimento {3:N1}%" -f $first.handles, $last.handles, $maxHandles, $handleGrowth)
Write-Host ("  cpu_pct:  maximo {0:N1}% (media do processo / nucleos)" -f $maxCpu)
Write-Host ("  csv:      {0}" -f $Out)

$failed = $false
if ($wsGrowth -gt 5) {
    Write-Warning ("memoria cresceu {0:N1} MB (limite 5 MB)" -f $wsGrowth)
    $failed = $true
}
if ([math]::Abs($handleGrowth) -gt 5) {
    Write-Warning ("handles variaram {0:N1}% (limite 5%)" -f $handleGrowth)
    $failed = $true
}
if ($failed) { exit 1 }
exit 0
