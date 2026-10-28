#Requires -Version 5.1
<#
.SYNOPSIS
  Coleta automatizada do spike S5 (modo A|B).

  Sobe o protótipo em modo auto (--auto <modo> --seconds N --json <arq>),
  amostra a cada 1 s:
    - CPU do processo (delta de segundos de CPU, em % de 1 núcleo)
    - memória (working set) do processo principal
    - CPU da árvore (principal + filhos msedgewebview2 — é onde o decode roda)
    - GPU da árvore via \GPU Engine(*)\Utilization Percentage, filtrando as
      instâncias "pid_<PID>_..." (soma dos engines do processo)

  NOTA DE ARQUITETURA: o sampler de GPU roda em um processo filho separado.
  Chamar Get-Counter (PDH) no MESMO processo que faz Get-Process/Stop-Process
  torna o app invisível para a enumeração de processos (comportamento do
  provider de GPU nesta máquina) — por isso o PDH fica isolado no filho.

  Ao final escreve docs\spikes\tmp\s5-<modo>-<dpi>.csv e imprime uma linha
  RESUMO (fps, descartes, MB/s, latência p50/p95 do JSON do app, CPU/GPU médias).

  IMPORTANTE: -Dpi é APENAS um rótulo da execução. O script NÃO altera a
  escala do Windows (nenhuma escrita em registro). Mude a escala em
  Configurações -> Tela antes de cada grupo de execuções (ver ROTEIRO-HUMANO.md).

.EXAMPLE
  .\run-s5.ps1 -Mode A -Dpi 100
.EXAMPLE
  .\run-s5.ps1 -Mode B -Dpi 150 -Minutes 10
.EXAMPLE
  .\run-s5.ps1 -Mode A -Dpi 100 -Seconds 10   # teste curto (agentes)
#>
[CmdletBinding()]
param(
    [Parameter(Mandatory)][ValidateSet('A', 'B')][string]$Mode,
    [ValidateSet(100, 150, 200)][int]$Dpi = 100,
    [ValidateRange(1, 120)][int]$Minutes = 10,
    [ValidateRange(0, 3600)][int]$Seconds = 0
)
$ErrorActionPreference = 'Stop'
# Números sempre em InvariantCulture (CSV e RESUMO com ponto decimal).
[System.Threading.Thread]::CurrentThread.CurrentCulture = [System.Globalization.CultureInfo]::InvariantCulture

# Resolve-Path: $PSScriptRoot pode ser relativo conforme o CWD de quem chama.
# spikes/s5/<script> → 2 níveis acima é a raiz do repositório.
$spikeDir = (Resolve-Path $PSScriptRoot).Path
$repoRoot = Split-Path -Parent (Split-Path -Parent $spikeDir)
$exe      = Join-Path $spikeDir 'src-tauri\target\debug\s5-spike.exe'
$distIdx  = Join-Path $spikeDir 'dist\index.html'

if ($Seconds -gt 0) { $duration = $Seconds } else { $duration = $Minutes * 60 }
$jsonPath = Join-Path $spikeDir "auto-$Mode-$Dpi.json"
$csvOut   = Join-Path $repoRoot "docs\spikes\tmp\s5-$Mode-$Dpi.csv"
$gpuCsv   = Join-Path $env:TEMP "s5-gpu-$Mode-$Dpi-$PID.csv"

$tree     = @()
$mainProc = $null
$gpuProc  = $null
$gpuErr   = $null

function Stop-Tree([array]$pids) {
    foreach ($p in $pids) {
        Stop-Process -Id $p -Force -ErrorAction SilentlyContinue
    }
}

# Descendentes msedgewebview2 (até 3 níveis) a partir de $rootPid. O WebView2
# usa um pool de processos: o renderer que desenha pode nascer após o snapshot
# inicial — por isso o loop refaz a busca e soma os PIDs novos.
function Get-WebviewTree([int]$rootPid) {
    $found = New-Object System.Collections.Generic.List[int]
    $found.Add($rootPid)
    $current = @($rootPid)
    for ($lvl = 0; $lvl -lt 3; $lvl++) {
        $next = @()
        foreach ($pp in $current) {
            $next += @(Get-CimInstance Win32_Process -Filter "ParentProcessId=$pp" -ErrorAction SilentlyContinue |
                Where-Object { $_.Name -eq 'msedgewebview2.exe' } |
                Select-Object -ExpandProperty ProcessId)
        }
        foreach ($cpid in $next) {
            if ($null -ne $cpid -and -not $found.Contains([int]$cpid)) { $found.Add([int]$cpid) }
        }
        if ($next.Count -eq 0) { break }
        $current = $next
    }
    return $found
}

try {
    # --- 0. Não deixa execuções antigas abertas -------------------------------
    Get-Process -Name 's5-spike' -ErrorAction SilentlyContinue | ForEach-Object {
        Write-Warning "s5-spike.exe ainda aberto (pid $($_.Id)); encerrando."
        Stop-Process -Id $_.Id -Force
    }

    # --- 1. Build se necessário (exe embeda o dist; dist mais novo -> rebuild) --
    if (-not (Test-Path $distIdx)) {
        Write-Host 'dist/ ausente: pnpm build...'
        Push-Location $spikeDir
        & pnpm build 2>&1 | Select-Object -Last 3
        Pop-Location
        if ($LASTEXITCODE -ne 0) { throw 'pnpm build falhou' }
    }
    if (-not (Test-Path $exe) -or
        ((Get-Item $distIdx).LastWriteTime -gt (Get-Item $exe).LastWriteTime)) {
        Write-Host 'exe ausente/antigo: cargo build...'
        Push-Location (Join-Path $spikeDir 'src-tauri')
        & cargo build 2>&1 | Select-Object -Last 3
        Pop-Location
        if ($LASTEXITCODE -ne 0) { throw 'cargo build falhou' }
    }
    Remove-Item $jsonPath -ErrorAction SilentlyContinue

    # --- 2. Sobe o app em modo auto (CWD = raiz do repo: fixtures/ é relativo) --
    $argsApp = @('--auto', $Mode, '--seconds', [string]$duration, '--json', $jsonPath)
    $mainProc = Start-Process -FilePath $exe -ArgumentList $argsApp `
        -WorkingDirectory $repoRoot -PassThru
    $mainPid = $mainProc.Id
    Write-Host ("app pid {0} | modo {1} | dpi {2} (rótulo) | duração {3}s | json {4}" -f $mainPid, $Mode, $Dpi, $duration, $jsonPath)

    # --- 3. Sampler de GPU em processo filho (PDH isolado; ver nota acima) ----
    # gpu-sample.ps1 escreve "START|<unix-ms>" e, 1/s, "idx|instância|valor|<unix-ms>".
    # (Subprocesso com -File: evita quoting em -Command e isola o PDH.)
    $maxSamples = $duration + 20
    $gpuErr   = Join-Path $env:TEMP "s5-gpu-$Mode-$Dpi-$PID.err"
    $gpuArgs  = @('-NoProfile', '-ExecutionPolicy', 'Bypass', '-File',
        (Join-Path $spikeDir 'gpu-sample.ps1'),
        '-Csv', $gpuCsv, '-MaxSamples', "$maxSamples")
    $gpuProc = Start-Process -FilePath powershell.exe `
        -ArgumentList $gpuArgs `
        -WindowStyle Hidden -RedirectStandardError $gpuErr -PassThru

    # --- 4. Espera WebView2 + auto-início; monta a árvore de processos ---------
    Start-Sleep -Seconds 5
    $tree = Get-WebviewTree $mainPid
    Write-Host ("árvore de processos: {0}" -f (($tree -join ', ')))

    # --- 5. Loop de amostragem: CPU + memória a cada 1 s ------------------------
    $loopStartMs = [DateTimeOffset]::Now.ToUnixTimeMilliseconds()
    $prevCpu = @{}
    foreach ($p in $tree) {
        try { $prevCpu[$p] = [double](Get-Process -Id $p).CPU } catch { $prevCpu[$p] = 0.0 }
    }
    $rows = New-Object System.Collections.Generic.List[object]
    for ($t = 0; $t -le $duration; $t++) {
        Start-Sleep -Seconds 1
        # O app (auto) pode encerrar antes do fim do loop (o timer dele começa
        # após a inicialização). Se já saiu, mata o sampler de GPU agora: amostrar
        # depois do desaparecimento dos engines gera "amostra inválida" (aviso
        # benigno do PDH que polui o stderr).
        if ($mainProc.HasExited -and $null -ne $gpuProc -and -not $gpuProc.HasExited) {
            Stop-Process -Id $gpuProc.Id -Force -ErrorAction SilentlyContinue
        }
        # Pool do WebView2: renderers podem nascer após o snapshot. Atualiza a
        # árvore a cada 5 s (PIDs novos entram com baseline de CPU para não
        # estourar o cpu_tree).
        if ($t -gt 0 -and $t % 5 -eq 0 -and -not $mainProc.HasExited) {
            foreach ($np in (Get-WebviewTree $mainPid)) {
                if (-not $tree.Contains([int]$np)) {
                    $tree.Add([int]$np)
                    try { $prevCpu[$np] = [double](Get-Process -Id $np).CPU } catch { $prevCpu[$np] = 0.0 }
                }
            }
        }
        $cpuMain = 0.0
        $cpuTree = 0.0
        $memMb = 0.0
        $nowCpu = @{}
        foreach ($p in $tree) {
            try {
                $pr = Get-Process -Id $p
                $cur = [double]$pr.CPU
                $d = $cur - ($prevCpu[$p])
                if ($d -lt 0) { $d = 0.0 }
                $cpuTree += $d
                if ($p -eq $mainPid) {
                    $cpuMain = $d
                    $memMb = [math]::Round($pr.WorkingSet64 / 1MB, 0)
                }
                $nowCpu[$p] = $cur
            } catch {
                # processo filho pode sair durante a execução
                $nowCpu[$p] = $prevCpu[$p]
            }
        }
        $prevCpu = $nowCpu
        $rows.Add([pscustomobject]@{
            t       = $t
            cpuMain = [math]::Round($cpuMain * 100, 1)
            memMb   = $memMb
            cpuTree = [math]::Round($cpuTree * 100, 1)
        })
        if ($t % 30 -eq 0) {
            Write-Host ("  t={0,4}s  cpu_main={1,5}%  cpu_tree={2,5}%  mem={3} MB" -f $t, $rows[$t].cpuMain, $rows[$t].cpuTree, $rows[$t].memMb)
        }
    }

    # --- 6. GPU: soma das instâncias pid_<PID> da árvore, alinhado por tempo ----
    # Cada amostra GPU cobre a janela de ~1 s antes do seu timestamp; a linha t
    # do loop cobre [t-1, t] s a partir do início do loop.
    $gpuPerT = @{}
    $gpuSeen = $false
    if ($null -ne $gpuProc) {
        if (-not $gpuProc.HasExited) { Stop-Process -Id $gpuProc.Id -Force -ErrorAction SilentlyContinue }
        Start-Sleep -Seconds 1
    }
    $treeStr = '|' + (($tree | ForEach-Object { [string]$_ }) -join '|') + '|'
    if (Test-Path $gpuCsv) {
        foreach ($line in (Get-Content $gpuCsv)) {
            if (-not $line -or $line.StartsWith('START|')) { continue }
            $parts = $line.Split('|')
            if ($parts.Count -lt 4) { continue }
            $m = [regex]::Match($parts[1], '^pid_(\d+)_')
            if (-not $m.Success) { continue }  # instância sem PID -> não atribui
            if (-not $treeStr.Contains('|' + $m.Groups[1].Value + '|')) { continue }
            $val = 0.0
            if (-not [double]::TryParse($parts[2], [System.Globalization.NumberStyles]::Float,
                    [System.Globalization.CultureInfo]::InvariantCulture, [ref]$val)) { continue }
            $tsMs = 0L
            if (-not [long]::TryParse($parts[3], [ref]$tsMs)) { continue }
            $t = [int][math]::Floor(($tsMs - $loopStartMs) / 1000)
            if ($t -lt 0 -or $t -gt $duration) { continue }
            if ($gpuPerT.ContainsKey($t)) { $gpuPerT[$t] += $val } else { $gpuPerT[$t] = $val }
            $gpuSeen = $true
        }
    }

    # --- 7. CSV de saída --------------------------------------------------------
    New-Item -ItemType Directory -Force (Split-Path $csvOut -Parent) | Out-Null
    $csvLines = @('t_s,cpu_main_pct,mem_mb,cpu_tree_pct,gpu_pct')
    foreach ($r in $rows) {
        $g = ''
        if ($gpuPerT.ContainsKey($r.t)) {
            $g = [string]([math]::Round($gpuPerT[$r.t], 1))
        }
        $csvLines += ('{0},{1},{2},{3},{4}' -f $r.t, $r.cpuMain, $r.memMb, $r.cpuTree, $g)
    }
    Set-Content -Path $csvOut -Value ($csvLines -join [Environment]::NewLine) -Encoding ASCII

    # --- 8. App encerra sozinho (auto_finish); confirma e força se preciso ------
    try { $mainProc.WaitForExit(60000) } catch { }
    $appExit = $null
    if ($mainProc.HasExited) {
        $appExit = $mainProc.ExitCode
    } else {
        Write-Warning 'app não encerrou sozinho; forçando.'
        Stop-Tree $tree
        try { $mainProc.WaitForExit(10000) } catch { }
    }
    if ($appExit -ne 0) { Write-Warning ("código de saída do app: {0}" -f $appExit) }

    # --- 9. Resumo ---------------------------------------------------------------
    $cpuMainAvg = [math]::Round((($rows | Measure-Object -Property cpuMain -Average).Average), 1)
    $cpuTreeAvg = [math]::Round((($rows | Measure-Object -Property cpuTree -Average).Average), 1)
    $gpuVals = @($rows | ForEach-Object {
        if ($gpuPerT.ContainsKey($_.t)) { [double]$gpuPerT[$_.t] }
    } | Where-Object { $null -ne $_ })
    $gpuErrText = ''
    if (Test-Path $gpuErr) {
        # Remove o bloco benigno do PDH "dados inválidos" (emite quando o alvo
        # sai durante o sampling; rala raça de 1 amostra). Outros erros permanecem.
        $raw = @(Get-Content $gpuErr -ErrorAction SilentlyContinue | Where-Object { $_ })
        $keep = New-Object System.Collections.Generic.List[string]
        $i = 0
        while ($i -lt $raw.Count) {
            if ($raw[$i] -match 'Get-Counter : (Os dados em uma das amostras|.*invalid data)') {
                while ($i -lt $raw.Count -and $raw[$i] -notmatch '^\s*\+\s') { $i++ }
                $i++
                continue
            }
            $keep.Add($raw[$i]); $i++
        }
        $gpuErrText = ($keep | Select-Object -First 3) -join ' | '
    }
    if ($gpuVals.Count -gt 0) {
        $gpuAvg = [string]([math]::Round((($gpuVals | Measure-Object -Average).Average), 1))
    } elseif (-not (Test-Path $gpuCsv)) {
        $gpuAvg = 'n/a (sampler de GPU não produziu dados — contador indisponível)'
    } elseif (-not $gpuSeen) {
        $gpuAvg = 'n/a (nenhuma instância pid_<PID> da árvore no período)'
    } else {
        $gpuAvg = 'n/a'
    }
    if ($gpuErrText) { $gpuAvg += ' [' + $gpuErrText + ']' }

    $fps = 'n/a'; $drops = 'n/a'; $mbps = 'n/a'
    $lat50 = 'n/a'; $lat95 = 'n/a'; $memS = 'n/a'; $memE = 'n/a'
    if (Test-Path $jsonPath) {
        $j = Get-Content $jsonPath -Raw | ConvertFrom-Json
        $fps   = [string][math]::Round($j.fps, 2)
        $drops = [string]$j.drops
        $mbps  = [string][math]::Round($j.mbps, 3)
        $lat50 = [string][math]::Round($j.latency_ms.p50, 0)
        $lat95 = [string][math]::Round($j.latency_ms.p95, 0)
        $memS  = [string]$j.mem_mb.start
        $memE  = [string]$j.mem_mb.end
        if ($j.error) { Write-Warning ("JSON do app com error: {0}" -f $j.error) }
    } else {
        Write-Warning 'JSON do app ausente (app falhou antes do fim da medição)'
    }

    Write-Host ''
    Write-Host ("RESUMO s5 | mode={0} dpi={1} dur={2}s | fps={3} drops={4} mbps={5} | lat p50={6}ms p95={7}ms | cpu_main={8}% cpu_tree={9}% | gpu={10}% | mem {11}->{12}MB | csv={13} json={14}" -f `
        $Mode, $Dpi, $duration, $fps, $drops, $mbps, $lat50, $lat95, $cpuMainAvg, $cpuTreeAvg, $gpuAvg, $memS, $memE, $csvOut, $jsonPath)
}
finally {
    if ($null -ne $gpuProc) { Stop-Process -Id $gpuProc.Id -Force -ErrorAction SilentlyContinue }
    Stop-Tree $tree
    if (Test-Path $gpuCsv) { Remove-Item $gpuCsv -Force -ErrorAction SilentlyContinue }
    if (Test-Path $gpuErr) { Remove-Item $gpuErr -Force -ErrorAction SilentlyContinue }
}

# Confirma que nada ficou aberto (a árvore da execução)
$left = @($tree | Where-Object {
    $null -ne (Get-Process -Id $_ -ErrorAction SilentlyContinue)
})
if ($left.Count -gt 0) {
    Write-Warning ("processos restantes (forçando): {0}" -f ($left -join ', '))
    Stop-Tree $left
}
