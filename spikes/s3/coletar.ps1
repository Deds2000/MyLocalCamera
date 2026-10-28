# Coleta o log do spike S3 do aparelho via adb.
# Uso:
#   .\coletar.ps1 -Test T1 -Seconds 600   (dispara o teste, espera e salva o log)
#   .\coletar.ps1 -Device <serial>        (aparelho específico, se houver mais de um)
#   .\coletar.ps1                         (só salva o log atual, sem disparar teste)
param(
    [string]$Test = "",
    [int]$Seconds = 0,
    [string]$Device = ""
)

$ErrorActionPreference = "Stop"

if ($Device) { $adb = @("adb", "-s", $Device) } else { $adb = @("adb") }

$repoRoot = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
$tmp = Join-Path $repoRoot "docs\spikes\tmp"
New-Item -ItemType Directory -Force -Path $tmp | Out-Null

& $adb logcat -c
if ($LASTEXITCODE -ne 0) { throw "adb logcat -c falhou (aparelho conectado?)" }

if (-not [string]::IsNullOrWhiteSpace($Test)) {
    $sec = if ($Seconds -le 0) { 600 } else { $Seconds }
    & $adb shell am start -n com.localcamera.spike.s3/.MainActivity --es test $Test --ei seconds $sec
    if ($LASTEXITCODE -ne 0) { throw "am start falhou (APK instalado?)" }
    Write-Host "Teste $Test por ${sec}s em andamento..."
    Start-Sleep -Seconds ($sec + 15)
}

$log = Join-Path $tmp "s3-log.txt"
& $adb logcat -d -s S3 | Out-File -FilePath $log -Encoding utf8
Write-Host "Log salvo em $log"
