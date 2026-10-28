#Requires -Version 5.1
<#
.SYNOPSIS
  Sampler de GPU isolado (subprocesso do run-s5.ps1).

  Por que um processo separado: chamar Get-Counter (PDH) no MESMO processo que
  faz Get-Process/Stop-Process torna o app invisível para a enumeração de
  processos nesta máquina (comportamento do provider de GPU). Aqui só existe PDH.

  Saída (arquivo texto, 1 linha por amostra/instância):
    START|<unix-ms>                 (antes da primeira amostra)
    <idx>|<instância>|<valor>|<unix-ms>
  onde <instância> é do tipo "pid_<PID>_luid_..._engtype_3d" e <valor> é o
  "Utilization Percentage" (InvariantCulture, ponto decimal). <unix-ms> é o
  instante em que a amostra chegou (janela coberta: ~1 s anterior).
  Flush a cada linha: se o processo-pai matar o sampler, nada se perde.
#>
[CmdletBinding()]
param(
    [Parameter(Mandatory)][string]$Csv,
    [Parameter(Mandatory)][int]$MaxSamples
)
$ErrorActionPreference = 'Stop'
[System.Threading.Thread]::CurrentThread.CurrentCulture = [System.Globalization.CultureInfo]::InvariantCulture

# --- 1. Descoberta do set de contadores (nomes en/pt-BR) ---------------------
$set = $null
foreach ($n in @('GPU Engine', '*GPU Engine*', '*Engine da GPU*')) {
    $set = Get-Counter -ListSet $n -ErrorAction SilentlyContinue
    if ($set) { break }
}
if (-not $set) { exit 0 }   # sem set de GPU -> pai registra n/a com motivo

$utilNames = @('Utilization Percentage', 'Porcentagem de utilização',
               'Porcentagem de uso', 'Uso')
$paths = @($set.Paths)
$cp = $paths | Where-Object { $utilNames -contains (($_ -split '\\')[-1]) } |
    Select-Object -First 1
if (-not $cp) { $cp = $paths[0] }

# --- 2. Amostragem 1/s com timestamp e flush por linha -----------------------
$f = [System.IO.File]::CreateText($Csv)
try {
    $f.WriteLine('START|' + [DateTimeOffset]::Now.ToUnixTimeMilliseconds())
    $i = 0
    # Quando um engine do alvo some entre amostras, o PDH emite "dados inválidos"
    # (aviso + ErrorRecord CounterApiError). É benigno (os samples válidos seguem),
    # mas com $ErrorActionPreference=Stop mataria o sampler — por isso SilentlyContinue.
    Get-Counter -Counter $cp -SampleInterval 1 -MaxSamples $MaxSamples `
        -WarningAction SilentlyContinue -ErrorAction SilentlyContinue |
        ForEach-Object {
            $i = $i + 1
            $ts = [DateTimeOffset]::Now.ToUnixTimeMilliseconds()
            foreach ($c in $_.CounterSamples) {
                $f.WriteLine(('{0}|{1}|{2}|{3}' -f $i, $c.InstanceName, $c.CookedValue, $ts))
                $f.Flush()
            }
        }
} finally {
    $f.Dispose()
}
