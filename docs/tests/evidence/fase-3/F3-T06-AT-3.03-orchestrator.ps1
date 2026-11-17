[CmdletBinding()]
param()
$ErrorActionPreference='Stop'
$root='C:\Users\PC\AppData\Local\Temp\opencode'
$repo='D:\Repositorios\MyLocalCamera'
$ev=Join-Path $repo 'docs\tests\evidence\fase-3'
$helper=Join-Path $root 'F3-T06-runtime.ps1'
$at=Join-Path $root 'lc-f3t06\AT-3.03.ps1'
$cleanup=Join-Path $root 'lc-f3t06\AT-3.03-cleanup.ps1'
$state=Join-Path $root 'F3-T06-runtime.json'
$desktop=$null;$vite=$null;$atResult='NOT_STARTED';$cleanupResult='NOT_STARTED';$failureStage=$null;$atInvoked=$false
foreach($n in @('F3-T06-AT-3.03-approved.json','F3-T06-AT-3.03-attempt-4.json','F3-T06-AT-3.03-v2.ps1')){
 if(Test-Path (Join-Path $ev $n)){throw "evidence-destination-exists:$n"}
}
function Stage([string]$name){Write-Host ("STAGE "+$name) -ForegroundColor Cyan}
function Start-Ticks($value){if($value -is [datetime]){return $value.ToUniversalTime().Ticks};return [DateTimeOffset]::Parse([string]$value).UtcDateTime.Ticks}
function Add-Metadata([string]$path){
 if(-not (Test-Path $path)){return}
 $j=Get-Content -Raw $path|ConvertFrom-Json
 $j|Add-Member -NotePropertyName sourceSha256 -NotePropertyValue ([ordered]@{
  testedHarness=(Get-FileHash $at -Algorithm SHA256).Hash
  cleanupHelper=(Get-FileHash $cleanup -Algorithm SHA256).Hash
  runtimeHelper=(Get-FileHash $helper -Algorithm SHA256).Hash
  mockSender=(Get-FileHash (Join-Path $repo 'target\debug\mock-sender.exe') -Algorithm SHA256).Hash
 }) -Force
 $j|Add-Member -NotePropertyName versionInfo -NotePropertyValue ([ordered]@{
  mockSender=(Get-Item (Join-Path $repo 'target\debug\mock-sender.exe')).VersionInfo.FileVersion
  desktopExe=(Get-Item (Join-Path $repo 'target\debug\desktop.exe')).VersionInfo.FileVersion
  os=[Environment]::OSVersion.VersionString
 }) -Force
 $j|ConvertTo-Json -Depth 8|Set-Content -Encoding utf8 $path
}
try{
 Stage 'preflight-and-release-stale-manager-state'
 if(Test-Path $state){
  $old=Get-Content -Raw $state|ConvertFrom-Json
  $oldLive=@();foreach($entry in @($old.desktop,$old.vite)){if($entry -and (Get-Process -Id $entry.id -ErrorAction SilentlyContinue)){$oldLive+=1}}
  if($oldLive.Count){throw 'stale-management-state-has-live-process-reference;refusing-helper-stop'}
  & $helper -Action stop|Out-Null
 }
 Stage 'start-owned-runtime'
 $startJson=& $helper -Action start
 $started=$startJson|ConvertFrom-Json
 $desktop=Get-Process -Id $started.desktop.id -ErrorAction Stop
 $vite=Get-Process -Id $started.vite.id -ErrorAction Stop
 if($desktop.ProcessName -ne 'desktop' -or $vite.ProcessName -ne 'node'){throw 'helper-process-identity-mismatch'}
 $dTicks=$desktop.StartTime.ToUniversalTime().Ticks;$vTicks=$vite.StartTime.ToUniversalTime().Ticks
 if($started.desktop.started -and (Start-Ticks $started.desktop.started) -ne $dTicks){throw 'desktop-start-timestamp-mismatch'}
 if($started.vite.started -and (Start-Ticks $started.vite.started) -ne $vTicks){throw 'vite-start-timestamp-mismatch'}
 $cdpReady=$false;$watch=[Diagnostics.Stopwatch]::StartNew();while($watch.Elapsed.TotalSeconds -lt 20){
  try{$null=Invoke-RestMethod 'http://127.0.0.1:9226/json' -TimeoutSec 2;$cdpReady=$true;break}catch{Start-Sleep -Milliseconds 250}
 }
 if(-not $cdpReady){throw 'new-owned-loopback-cdp-not-ready'}
 Stage 'cleanup-existing-test-device-only'
 $cleanupPath=Join-Path $ev 'F3-T06-AT-3.03-cleanup-completed.json'
 if(Test-Path $cleanupPath){
  $prior=Get-Content -Raw $cleanupPath|ConvertFrom-Json
  if($prior.result -ne 'CLEANUP_CONFIRMED' -or $prior.deviceName -cne 'F3-T06-8c7df0d3'){throw 'prior-owned-device-cleanup-evidence-invalid'}
  $recheck=Join-Path $root 'lc-f3t06\cleanup-recheck.json'
  & $cleanup -DesktopPid $desktop.Id -CdpPort 9226 -DeviceName 'F3-T06-8c7df0d3' -Evidence $recheck|Out-Host
  $cj=Get-Content -Raw $recheck|ConvertFrom-Json;Remove-Item $recheck -Force
 }else{
  & $cleanup -DesktopPid $desktop.Id -CdpPort 9226 -DeviceName 'F3-T06-8c7df0d3' -Evidence $cleanupPath|Out-Host
  $cj=Get-Content -Raw $cleanupPath|ConvertFrom-Json
 }
 if($cj.result -notin @('CLEANUP_CONFIRMED','ALREADY_ABSENT') -or -not $cj.trustedListAfterSucceeded -or $cj.matchingTrustedRecordsAfter -ne 0){throw 'existing-device-cleanup-not-confirmed;no-new-pair'}
 $cleanupResult=$cj.result
 Stage 'run-single-approved-at-retry'
 $atInvoked=$true
 & $at -DesktopPid $desktop.Id -CdpPort 9226 -Repo $repo
 $approved=Join-Path $ev 'F3-T06-AT-3.03-approved.json';$attempt=Join-Path $ev 'F3-T06-AT-3.03-attempt-4.json'
 if(Test-Path $approved){$aj=Get-Content -Raw $approved|ConvertFrom-Json;if($aj.result -ne 'PASS'){throw 'approved-evidence-does-not-report-pass'};$atResult='PASS'}else{$atResult='BLOCKED';if(-not (Test-Path $attempt)){throw 'AT-runner-returned-without-approved-or-attempt4-evidence'}}
 if(Test-Path $approved){Add-Metadata $approved}else{Add-Metadata $attempt}
 Copy-Item $at (Join-Path $ev 'F3-T06-AT-3.03-v2.ps1')
 Stage 'finalize-at-evidence-and-clean-residual-entry'
 if(Test-Path $attempt){
  $a=Get-Content -Raw $attempt|ConvertFrom-Json
  if($a.ownedDeviceName -and $a.ownedTrustedDeviceCleanup -eq 'agent-cleanup-required'){
   $tempCleanup=Join-Path $root 'lc-f3t06\final-cleanup.json'
   & $cleanup -DesktopPid $desktop.Id -CdpPort 9226 -DeviceName $a.ownedDeviceName -Evidence $tempCleanup|Out-Host
   if((Get-Content -Raw $tempCleanup|ConvertFrom-Json).result -notin @('CLEANUP_CONFIRMED','ALREADY_ABSENT')){throw 'residual-owned-device-cleanup-blocked'}
   Remove-Item $tempCleanup -Force
  }
 }
}catch{
 $failureStage=$_.Exception.Message
 Write-Host ("BLOCKED stage="+$failureStage) -ForegroundColor Yellow
}finally{
 if($atInvoked){
  $archive=Join-Path $ev 'F3-T06-AT-3.03-v2.ps1'
  if(-not (Test-Path $archive)){Copy-Item $at $archive}
  $approved=Join-Path $ev 'F3-T06-AT-3.03-approved.json';$attempt=Join-Path $ev 'F3-T06-AT-3.03-attempt-4.json'
  if(Test-Path $approved){Add-Metadata $approved}elseif(Test-Path $attempt){Add-Metadata $attempt}
 }
 Stage 'stop-only-helper-owned-runtime'
 if(Test-Path $state){
  try{
   $owned=Get-Content -Raw $state|ConvertFrom-Json
   $sameD=$false;$sameV=$false
   $p=Get-Process -Id $owned.desktop.id -ErrorAction SilentlyContinue
   if($p -and $p.ProcessName -eq 'desktop'){$sameD=($p.StartTime.ToUniversalTime().Ticks -eq (Start-Ticks $owned.desktop.started))}
   $p=Get-Process -Id $owned.vite.id -ErrorAction SilentlyContinue
   if($p -and $p.ProcessName -eq 'node'){$sameV=($p.StartTime.ToUniversalTime().Ticks -eq (Start-Ticks $owned.vite.started))}
   if($sameD -or $sameV){& $helper -Action stop|Out-Host}else{Remove-Item $state -Force;Write-Host 'No owned processes remain; discarded own management state.'}
  }catch{Write-Host ("RUNTIME_CLEANUP_BLOCKED "+$_.Exception.Message) -ForegroundColor Red}
 }
 $left=@(Get-Process -Id $(if($desktop){$desktop.Id}else{-1}),$(if($vite){$vite.Id}else{-1}) -ErrorAction SilentlyContinue)
 $ports=@(Get-NetTCPConnection -State Listen -ErrorAction SilentlyContinue|Where-Object{$_.LocalPort -in @(1420,48650,9226) -and ($left.Id -contains $_.OwningProcess)})
 Write-Host ("FINAL ownedProcessCount="+$left.Count+" ownedPorts="+$ports.Count+" cleanup="+$cleanupResult+" AT="+$atResult)
 if($failureStage){Write-Host ('FINAL_BLOCK='+$failureStage)}
}
