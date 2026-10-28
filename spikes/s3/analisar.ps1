param(
    [string]$TmpDir = "docs/spikes/tmp"
)
$ErrorActionPreference = "Stop"
$repoRoot = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
if (-not [System.IO.Path]::IsPathRooted($TmpDir)) { $TmpDir = Join-Path $repoRoot $TmpDir }

# Agrupa as linhas CSV por label real (T1, T2, T3a, T3b, T4b, T4f).
$labels = @{}
$firstKf = @{}
$encName = @{}

foreach ($file in Get-ChildItem $TmpDir -Filter 's3-T*.txt' -ErrorAction SilentlyContinue | Sort-Object Name) {
    # Escopo por teste: s3-T3.txt só contribui labels T3a/T3b, etc. (evita linhas estranhas).
    $testNum = ($file.BaseName -replace '^s3-T', '') -replace '[a-z]$', ''
    foreach ($line in Get-Content $file.FullName) {
        if ($line -match 'S3\s*:\s*(T[1-4][a-z]?),(\d+),([\d.]+),(\d+),(\d+),([A-Za-z0-9._-]+)') {
            $label  = $Matches[1]
            $mMs    = [long]$Matches[2]
            $mFps   = [double]$Matches[3]
            $mBytes = [long]$Matches[4]
            $mKey   = [int]$Matches[5]
            if ($label.Substring(1,1) -ne $testNum) { continue }
            if (-not $labels.ContainsKey($label)) {
                $labels[$label] = New-Object System.Collections.Generic.List[object]
            }
            $labels[$label].Add([pscustomobject]@{
                Ms    = $mMs
                Fps   = $mFps
                Bytes = $mBytes
                Key   = $mKey
            })
            continue
        }
        if ($line -match 'S3\s*:\s*(T[1-4][a-z]?) timeToFirstKeyframe_ms=(\d+)') {
            $lab = $Matches[1]; $ms = [int]$Matches[2]
            if ($lab.Substring(1,1) -eq $testNum) { $firstKf[$lab] = $ms }
            continue
        }
        if ($line -match 'S3\s*:\s*(T[1-4][a-z]?) enc_name=([A-Za-z0-9._-]+)') {
            $lab = $Matches[1]; $enc = $Matches[2]
            if ($lab.Substring(1,1) -eq $testNum) { $encName[$lab] = $enc }
        }
    }
}

$rows = @()
foreach ($label in ($labels.Keys | Sort-Object)) {
    $samples = $labels[$label]
    if ($samples.Count -lt 2) { continue }

    $counted = $samples | Where-Object { $_.Ms -ge 5000 }
    if ($counted.Count -lt 2) { $counted = $samples }
    $fps = $counted | ForEach-Object { $_.Fps }
    $mean = ($fps | Measure-Object -Average).Average
    $variance = (($fps | ForEach-Object { [math]::Pow($_ - $mean, 2) } | Measure-Object -Sum).Sum) / $fps.Count
    $std = [math]::Sqrt($variance)
    $durMs = $samples[-1].Ms - $samples[0].Ms
    $bytesTotal = ($samples | Measure-Object -Property Bytes -Sum).Sum   # bytes/segundo -> soma
    $mbps = if ($durMs -gt 0) { ($bytesTotal * 8 / ($durMs / 1000.0)) / 1e6 } else { 0 }
    $keyframes = ($samples | Where-Object { $_.Key -gt 0 }).Count

    $rows += [pscustomobject]@{
        Label       = $label
        Amostras    = $samples.Count
        Duracao_s   = [math]::Round($durMs / 1000.0, 1)
        Fps_medio   = [math]::Round($mean, 1)
        Fps_desvio  = [math]::Round($std, 1)
        Fps_min     = ($fps | Measure-Object -Minimum).Minimum
        Fps_max     = ($fps | Measure-Object -Maximum).Maximum
        Keyframes   = $keyframes
        Primeiro_KF = $firstKf[$label]
        Mbps_medio  = [math]::Round($mbps, 2)
        Encoder     = $encName[$label]
    }
}

$rows | Format-Table -AutoSize
$md = Join-Path $TmpDir 's3-resumo.md'
if ($rows.Count -gt 0) {
    $rows | ForEach-Object {
        "| $($_.Label) | $($_.Fps_medio) | $($_.Fps_desvio) | $($_.Fps_min)-$($_.Fps_max) | $($_.Mbps_medio) | $($_.Keyframes) | $($_.Primeiro_KF) | $($_.Encoder) |"
    } | Set-Content -Path $md -Encoding utf8
}
Write-Host "Resumo em $md"
