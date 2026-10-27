<#
.SYNOPSIS  CI local do LocalCamera.
.PARAMETER Fast         lint + typecheck + testes JS + fmt/clippy (sem cargo test e sem Android)
.PARAMETER SkipAndroid  nao roda os testes do Android
.PARAMETER Only         all | ts | rust | android
.PARAMETER KeepGoing    continua apos falhas
#>
param(
  [switch]$Fast,
  [switch]$SkipAndroid,
  [ValidateSet('all','ts','rust','android')][string]$Only = 'all',
  [switch]$KeepGoing
)
$ErrorActionPreference = 'Stop'
Set-Location (Split-Path $PSScriptRoot -Parent)
$script:results = New-Object System.Collections.Generic.List[object]
$script:failed = $false

function Invoke-Step {
  param([string]$Name, [scriptblock]$Block)
  if ($script:failed -and -not $KeepGoing) {
    $script:results.Add([pscustomobject]@{ Step = $Name; Seconds = '-'; Result = 'SKIPPED' }); return
  }
  Write-Host "==> $Name" -ForegroundColor Cyan
  $global:LASTEXITCODE = 0
  $sw = [Diagnostics.Stopwatch]::StartNew()
  $ok = $true
  try { & $Block; if ($LASTEXITCODE -ne 0) { $ok = $false } }
  catch { Write-Host $_ -ForegroundColor Red; $ok = $false }
  $sw.Stop()
  if (-not $ok) { $script:failed = $true }
  $script:results.Add([pscustomobject]@{
    Step = $Name; Seconds = [math]::Round($sw.Elapsed.TotalSeconds, 1); Result = $(if ($ok) { 'OK' } else { 'FAIL' })
  })
}

$runTs      = $Only -in 'all','ts'
$runRust    = $Only -in 'all','rust'
$runAndroid = ($Only -in 'all','android') -and (-not $SkipAndroid) -and (-not $Fast)

if ($runTs) {
  Invoke-Step 'pnpm install'  { pnpm install --frozen-lockfile }
  Invoke-Step 'lint'          { pnpm lint }
  Invoke-Step 'typecheck'     { pnpm typecheck }
  Invoke-Step 'testes JS'     { pnpm test }
}
if ($runRust) {
  Invoke-Step 'cargo fmt'     { cargo fmt --all -- --check }
  Invoke-Step 'cargo clippy'  { cargo clippy --workspace --all-targets -- -D warnings }
  if (-not $Fast) { Invoke-Step 'cargo test' { cargo test --workspace } }
}
if ($runAndroid) {
  Invoke-Step 'android unit+lint' {
    Push-Location mobile/android
    try { .\gradlew.bat testDebugUnitTest lintDebug } finally { Pop-Location }
  }
}

Write-Host ''
$script:results | Format-Table -AutoSize
if ($script:failed) { Write-Host 'CI: FALHOU' -ForegroundColor Red; exit 1 }
Write-Host 'CI: OK' -ForegroundColor Green; exit 0
