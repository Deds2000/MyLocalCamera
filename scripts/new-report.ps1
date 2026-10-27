param([Parameter(Mandatory)][int]$Phase)
$ErrorActionPreference = 'Stop'
Set-Location (Split-Path $PSScriptRoot -Parent)
$dest = "docs/tests/fase-$Phase.md"
if (Test-Path $dest) { throw "$dest ja existe" }
$text = Get-Content docs/tests/_template.md -Raw -Encoding UTF8
$text = $text -replace 'Fase N', "Fase $Phase" -replace 'N\.(\d\d)', "$Phase.`$1"
$text = $text -replace 'AAAA-MM-DD', (Get-Date -Format 'yyyy-MM-dd')
$commit = (git rev-parse --short HEAD) 2>$null
$cpu = (Get-CimInstance Win32_Processor | Select-Object -First 1).Name
$gpu = (Get-CimInstance Win32_VideoController | Select-Object -ExpandProperty Name) -join ' / '
$ramGb = [math]::Round((Get-CimInstance Win32_ComputerSystem).TotalPhysicalMemory / 1GB)
$os = (Get-CimInstance Win32_OperatingSystem)
$sep = [char]0x00B7
$info = "$cpu $sep $gpu $sep $ramGb GB $sep Windows $($os.Version)"
$text = $text -replace '\| Commit / tag \| \|', "| Commit / tag | $commit |"
$text = $text -replace '\| PC \(CPU . GPU . RAM . Windows build\) \| \|', "| PC (CPU $sep GPU $sep RAM $sep Windows build) | $info |"
[System.IO.File]::WriteAllText((Join-Path $PWD $dest), $text, (New-Object System.Text.UTF8Encoding($false)))
Write-Host "Criado $dest"
