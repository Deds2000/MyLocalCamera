# Gera os fixtures de teste (video H.264 Annex-B + PCM) em fixtures/ (fora do Git).
# Uso: powershell -NoProfile -ExecutionPolicy Bypass -File scripts/make-fixtures.ps1 [-Force]
# Requer: ffmpeg (libx264) no PATH e cargo (workspace raiz) para o fixture-tool.
# Sem -Force, arquivos ja gerados sao mantidos ("ok (existe)"); -Force regenera tudo.
#
# Nota (ffmpeg 6.1): o contador de quadros na faixa usa enable='gte(mod(floor(n/POW),2),1)'
# com POW calculado no proprio script ([math]::Pow(2,k)) e escrito como literal no filtro.
# A forma n/2^k depende do eval de expressoes do ffmpeg e nao e usada para garantir
# portabilidade entre versoes.
param([switch]$Force)
$ErrorActionPreference = 'Stop'
Set-Location (Split-Path $PSScriptRoot -Parent)
if (-not (Get-Command ffmpeg -ErrorAction SilentlyContinue)) { throw 'ffmpeg not found in PATH' }
$dir = 'fixtures'; New-Item -ItemType Directory -Force $dir | Out-Null
$x264 = '-c:v libx264 -profile:v high -preset veryfast -bf 0 -g 30 -keyint_min 30 -tune zerolatency -pix_fmt yuv420p -x264-params aud=1:repeat-headers=1:scenecut=0'

function Need($name) { $p = Join-Path $dir $name; if ((Test-Path $p) -and -not $Force) { Write-Host "ok (existe) $name"; return $null }; return $p }

function New-PatternFile($name, [int]$w, [int]$h, [string[]]$colors, [int]$seconds) {
  $out = Need $name; if (-not $out) { return }
  $qw = $w / 2; $qh = $h / 2; $bw = [int]($w / 32); $bh = [int]($h / 27)
  $parts = @("drawbox=x=0:y=0:w=$($bw*16):h=${bh}:color=black:t=fill")
  0..15 | ForEach-Object {
    $parts += "drawbox=x=$($_*$bw):y=0:w=${bw}:h=${bh}:color=white:t=fill:enable='gte(mod(floor(n/$([math]::Pow(2,$_))),2),1)'"
  }
  $graph = "[0:v][1:v][2:v][3:v]xstack=inputs=4:layout=0_0|w0_0|0_h0|w0_h0[q];[q]" + ($parts -join ',') + "[out]"
  $script = Join-Path $env:TEMP 'lc-filter.txt'; Set-Content -Path $script -Value $graph -Encoding ascii
  $inputs = $colors | ForEach-Object { "-f lavfi -i color=c=${_}:s=${qw}x${qh}:r=30" }
  $cmd = "ffmpeg -y -loglevel error $($inputs -join ' ') -filter_complex_script `"$script`" -map [out] -t $seconds $x264 -f h264 `"$out`""
  Write-Host $cmd; Invoke-Expression $cmd
  if ($LASTEXITCODE -ne 0) { throw "ffmpeg falhou (exit $LASTEXITCODE) para $out" }
}

# hd30.h264: testsrc2 1080p30, 10 s
$out = Need 'hd30.h264'
if ($out) {
  $cmd = "ffmpeg -y -loglevel error -f lavfi -i testsrc2=size=1920x1080:rate=30 -t 10 $x264 -f h264 `"$out`""
  Write-Host $cmd; Invoke-Expression $cmd
  if ($LASTEXITCODE -ne 0) { throw "ffmpeg falhou (exit $LASTEXITCODE) para $out" }
}

# pattern1080 / pattern720 (cores trocadas) / pattern43 (1440x1080, 10 s)
New-PatternFile 'pattern1080.h264' 1920 1080 @('red','green','blue','yellow') 20
New-PatternFile 'pattern720.h264' 1280 720 @('yellow','blue','green','red') 20
New-PatternFile 'pattern43.h264' 1440 1080 @('red','green','blue','yellow') 10

# sps-change.h264: 5 s de 720p + 5 s de 1080p (novo SPS/PPS no meio), via concat.
# Os temporarios ficam em fixtures/ e sao removidos apos o concat (o gitignore ja cobre).
$out = Need 'sps-change.h264'
if ($out) {
  Remove-Item "fixtures/p720_5s.h264", "fixtures/p1080_5s.h264" -ErrorAction SilentlyContinue
  New-PatternFile 'p720_5s.h264' 1280 720 @('yellow','blue','green','red') 5
  New-PatternFile 'p1080_5s.h264' 1920 1080 @('red','green','blue','yellow') 5
  cargo run -q -p fixture-tool -- concat "$out" "fixtures/p720_5s.h264" "fixtures/p1080_5s.h264"
  if ($LASTEXITCODE -ne 0) { throw "fixture-tool concat falhou (exit $LASTEXITCODE)" }
  Remove-Item "fixtures/p720_5s.h264", "fixtures/p1080_5s.h264" -ErrorAction SilentlyContinue
}

# corrupt.h264: pattern1080 com o AU 45 corrompido
$out = Need 'corrupt.h264'
if ($out) {
  cargo run -q -p fixture-tool -- corrupt "fixtures/pattern1080.h264" "$out" 45
  if ($LASTEXITCODE -ne 0) { throw "fixture-tool corrupt falhou (exit $LASTEXITCODE)" }
}

# no-idr-start.h264: pattern1080 sem o primeiro AU (comeca em quadro P)
$out = Need 'no-idr-start.h264'
if ($out) {
  cargo run -q -p fixture-tool -- drop-first-au "fixtures/pattern1080.h264" "$out"
  if ($LASTEXITCODE -ne 0) { throw "fixture-tool drop-first-au falhou (exit $LASTEXITCODE)" }
}

# tone440.pcm: seno 440 Hz, 48 kHz, 16 bits, mono, 30 s
$out = Need 'tone440.pcm'
if ($out) {
  $cmd = "ffmpeg -y -loglevel error -f lavfi -i ""sine=frequency=440:sample_rate=48000"" -t 30 -ac 1 -f s16le -acodec pcm_s16le `"$out`""
  Write-Host $cmd; Invoke-Expression $cmd
  if ($LASTEXITCODE -ne 0) { throw "ffmpeg falhou (exit $LASTEXITCODE) para $out" }
}

Get-ChildItem $dir -File | Where-Object Name -ne '.gitkeep' | ForEach-Object {
  "{0}  {1,10:N0} bytes  {2}" -f $_.Name, $_.Length, (Get-FileHash $_.FullName -Algorithm SHA256).Hash.Substring(0,12)
}
