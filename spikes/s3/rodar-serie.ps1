# Roda a serie T1..T4 do spike S3 em sequencia, salvando um log por teste.
# Uso:
#   .\rodar-serie.ps1 -Device RXCT800A2HR              (10 min por teste)
#   .\rodar-serie.ps1 -Device RXCT800A2HR -Seconds 60  (teste curto)
param(
    [string]$Device = "",
    [int]$Seconds = 600,
    [string[]]$Tests = @("T1", "T2", "T3", "T4")
)

$ErrorActionPreference = "Stop"

$repoRoot = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
$tmp = Join-Path $repoRoot "docs\spikes\tmp"
New-Item -ItemType Directory -Force -Path $tmp | Out-Null

$adbArgs = @()
if ($Device) { $adbArgs = @("-s", $Device) }

$envFile = Join-Path $tmp "s3-env.txt"
"inicio $(Get-Date -Format o)" | Add-Content $envFile

foreach ($t in $Tests) {
    Write-Host "=== $t start $(Get-Date -Format HH:mm:ss) ==="
    & adb @adbArgs shell am force-stop com.localcamera.spike.s3
    & adb @adbArgs logcat -c
    & adb @adbArgs shell am start -n com.localcamera.spike.s3/.MainActivity --es test $t --ei seconds $Seconds | Out-Null
    Start-Sleep -Seconds ($Seconds + 20)

    $log = Join-Path $tmp "s3-$t.txt"
    & adb @adbArgs logcat -d -s S3 | Out-File -FilePath $log -Encoding utf8
    & adb @adbArgs shell am force-stop com.localcamera.spike.s3

    $bat = (& adb @adbArgs shell dumpsys battery | Select-String "temperature").Line
    $therm = (& adb @adbArgs shell dumpsys thermalservice 2>&1 |
        Select-String "Temperature\{" | Select-Object -First 4 |
        ForEach-Object { $_.Line.Trim() }) -join " | "
    "$t fim $(Get-Date -Format HH:mm:ss) $($bat -replace '\s+', ' ') $therm" | Add-Content $envFile
    Write-Host "=== $t done ==="
}

"fim $(Get-Date -Format o)" | Add-Content $envFile
