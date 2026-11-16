[CmdletBinding()]
param([Parameter(Mandatory)][int]$DesktopPid,[Parameter(Mandatory)][ValidateRange(1,65535)][int]$CdpPort,
 [string]$Repo='D:\Repositorios\MyLocalCamera',[string]$MockExe='D:\Repositorios\MyLocalCamera\target\debug\mock-sender.exe',
 [string]$EvidenceDir='D:\Repositorios\MyLocalCamera\docs\tests\evidence\fase-3')
$ErrorActionPreference='Stop'; Add-Type -AssemblyName UIAutomationClient; Add-Type -AssemblyName UIAutomationTypes; Add-Type -AssemblyName System.Drawing
$run=Join-Path $env:LOCALAPPDATA ('Temp\opencode\lc-f3t06\run-'+[guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $run -Force|Out-Null; $state=Join-Path $run 'state.json'; $started=[Collections.Generic.List[Diagnostics.Process]]::new(); $name='F3-T06-'+[guid]::NewGuid().ToString('N').Substring(0,8)
$script:stage='initialization';$script:outputs=@{};$script:firstExitCode=$null;$script:firstExitReason='unclassified';$script:cleanup='not-needed';$script:createdPermanent=$false;$script:mockInvocation=$null;$script:hostsNormalized=$false
function Need($v,[string]$safe){if(-not $v){throw $safe}}
function Get-UiTree($scope){$scope.FindAll([System.Windows.Automation.TreeScope]::Subtree,[System.Windows.Automation.Condition]::TrueCondition)}
function Find-Ui($scope,[System.Windows.Automation.ControlType]$type,[string]$name){@((Get-UiTree $scope)|Where-Object{$_.Current.ControlType -eq $type -and $_.Current.Name -eq $name})|Select-Object -First 1}
function Invoke-Ui($el){Need ($null -ne $el) 'Required UIA control not found';$el.GetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern).Invoke()}
function Get-Window {
 $p=Get-Process -Id $DesktopPid -ErrorAction Stop; Need ($p.ProcessName -eq 'desktop') 'DesktopPid is not desktop.exe'
 $ct=[System.Windows.Automation.PropertyCondition]::new([System.Windows.Automation.AutomationElement]::ControlTypeProperty,[System.Windows.Automation.ControlType]::Window)
 $pidc=[System.Windows.Automation.PropertyCondition]::new([System.Windows.Automation.AutomationElement]::ProcessIdProperty,$DesktopPid)
 $nc=[System.Windows.Automation.PropertyCondition]::new([System.Windows.Automation.AutomationElement]::NameProperty,'LocalCamera')
 $cond=[System.Windows.Automation.AndCondition]::new($ct,[System.Windows.Automation.AndCondition]::new($pidc,$nc))
 [System.Windows.Automation.AutomationElement]::RootElement.FindFirst([System.Windows.Automation.TreeScope]::Children,$cond)
}
function Get-CdpUri {
 $tcp=[Net.Sockets.TcpClient]::new('127.0.0.1',$CdpPort);$tcp.Close()
 $targets=Invoke-RestMethod -Uri "http://127.0.0.1:$CdpPort/json" -TimeoutSec 3
 $target=@($targets|Where-Object{$_.type -eq 'page' -and $_.webSocketDebuggerUrl -match '^ws://(127\.0\.0\.1|localhost):'})|Select-Object -First 1
 Need ($target) 'No loopback WebView2 CDP page'
 $cts=[Threading.CancellationTokenSource]::new([TimeSpan]::FromSeconds(6))
  $ws=[Net.WebSockets.ClientWebSocket]::new();$null=$ws.ConnectAsync([uri]$target.webSocketDebuggerUrl,$cts.Token).GetAwaiter().GetResult()
 $script:cdpId=0;$script:cdpWs=$ws
 $expr="window.__TAURI_INTERNALS__.invoke('get_connection_info').then(x=>x.qrcodeUri)"
 $script:cdpId++;$req=@{id=$script:cdpId;method='Runtime.evaluate';params=@{expression=$expr;awaitPromise=$true;returnByValue=$true}}|ConvertTo-Json -Depth 8 -Compress
   $bytes=[Text.Encoding]::UTF8.GetBytes($req);$null=$ws.SendAsync([ArraySegment[byte]]::new($bytes),[Net.WebSockets.WebSocketMessageType]::Text,$true,$cts.Token).GetAwaiter().GetResult()
  $buf=[byte[]]::new(32768);$reply=$null
  for($attempt=0;$attempt -lt 20 -and -not $reply;$attempt++){
   $ms=[IO.MemoryStream]::new();do{$res=$ws.ReceiveAsync([ArraySegment[byte]]::new($buf),$cts.Token).GetAwaiter().GetResult();$ms.Write($buf,0,$res.Count)}while(-not $res.EndOfMessage)
   $msg=[Text.Encoding]::UTF8.GetString($ms.ToArray())|ConvertFrom-Json
   if($msg.id -eq $script:cdpId){$reply=$msg}
  }
  $ws.Dispose();$cts.Dispose();Need ($reply) 'CDP response timeout';Need (-not $reply.error -and -not $reply.result.exceptionDetails) 'CDP evaluate failed'
  $uri=$reply.result.result.value; Need ($uri -is [string] -and $uri.StartsWith('localcamera://connect?')) 'CDP did not return pairing URI'
  $uri=[regex]::new('([?&]hosts=)[^&]*').Replace($uri,'${1}127.0.0.1',1)
  Need ($uri -match '[?&]hosts=127\.0\.0\.1(?:&|$)') 'Pairing hosts could not be normalized to loopback'
  $script:hostsNormalized=$true
  return $uri
}
function Start-Mock([string[]]$argv){
 $psi=[Diagnostics.ProcessStartInfo]::new();$psi.FileName=$MockExe;$psi.UseShellExecute=$false;$psi.CreateNoWindow=$true;$psi.RedirectStandardOutput=$true;$psi.RedirectStandardError=$true
 $psi.WorkingDirectory=$Repo
 $script:mockInvocation=[ordered]@{executable=[IO.Path]::GetFullPath($MockExe);workingDirectory=$psi.WorkingDirectory;argumentCount=$argv.Count;argumentFlags=@($argv|Where-Object{$_ -match '^--'});argumentValues='withheld'}
 foreach($a in $argv){[void]$psi.ArgumentList.Add($a)}
  $p=[Diagnostics.Process]::new();$p.StartInfo=$psi;[void]$p.Start();$started.Add($p)
  $script:outputs[$p.Id]=@{Out=$p.StandardOutput.ReadToEndAsync();Err=$p.StandardError.ReadToEndAsync()};return $p
 }
function Get-MockReason([string]$text){
 $enumMap=@{BadToken='bad_token';NotApproved='not_approved';Revoked='revoked';Busy='busy';VersionMismatch='version_mismatch';BadAudioTicket='bad_audio_ticket';Timeout='timeout';ProtocolError='protocol_error';Other='other'}
 foreach($entry in $enumMap.GetEnumerator()){if($text -match "(?i)(\b$($entry.Key)\b|\b$($entry.Value)\b)"){return $entry.Value}}
 if($text -match '(?i)--uri:'){return 'uri_parser_error'}
 if($text -match '(?i)nenhum host respondeu'){return 'no_host_responded'}
 if($text -match '(?i)conexão encerrada sem WELCOME'){return 'closed_before_welcome'}
 if($text -match '(?i)WELCOME sem sessionId'){return 'welcome_missing_session_id'}
 if($text -match '(?i)WELCOME sem audioTicket'){return 'welcome_missing_audio_ticket'}
 if($text -match '(?i)URI de pareamento inválida'){return 'invalid_pairing_uri'}
 if($text -match '(?i)versão de protocolo incompatível'){return 'incompatible_protocol'}
 if($text -match '(?i)configuração:'){return 'configuration_error'}
 if($text -match '(?i)vídeo:'){return 'video_input_error'}
 if($text -match '(?i)io:'){return 'io_error'}
 return 'unclassified'
}
function Wait-Exit($p,[int]$sec){if(-not $p.WaitForExit($sec*1000)){throw 'mock-sender timed out'};$p.WaitForExit();$o=$script:outputs[$p.Id];$out=$o.Out.GetAwaiter().GetResult();$err=$o.Err.GetAwaiter().GetResult();$script:outputs[$p.Id]=@{Out=$out;Err=$err};if($p.Id -eq $script:firstPid){$script:firstExitCode=$p.ExitCode;$script:firstExitReason=Get-MockReason ($out+"`n"+$err)};return $p.ExitCode}
function Has-Name($scope,[string]$n){@((Get-UiTree $scope)|Where-Object{$_.Current.Name -eq $n}).Count -gt 0}
function Wait-Name($scope,[string]$n,[bool]$present,[int]$seconds){$sw=[Diagnostics.Stopwatch]::StartNew();do{$has=Has-Name $scope $n;if($has -eq $present){return $true};Start-Sleep -Milliseconds 200}while($sw.Elapsed.TotalSeconds -lt $seconds);return $false}
function Find-ConfirmDialog($scope){
  $until=[DateTime]::UtcNow.AddSeconds(5)
  while([DateTime]::UtcNow -lt $until){
    $els=Get-UiTree $scope
    $cancel=@($els|Where-Object{$_.Current.ControlType -eq [System.Windows.Automation.ControlType]::Button -and $_.Current.Name -in @('Cancelar','Cancel')})|Select-Object -First 1
    if($cancel){
      $walker=[System.Windows.Automation.TreeWalker]::ControlViewWalker;$parent=$walker.GetParent($cancel);$depth=0;$windowRuntime=($scope.GetRuntimeId() -join '.')
      while($parent -and $depth -lt 20 -and (($parent.GetRuntimeId() -join '.') -ne $windowRuntime)){
        $names=@(Get-UiTree $parent|ForEach-Object{$_.Current.Name})
        if($parent.Current.ControlType -in @([System.Windows.Automation.ControlType]::Window,[System.Windows.Automation.ControlType]::Pane,[System.Windows.Automation.ControlType]::Group,[System.Windows.Automation.ControlType]::Custom) -and ($names -contains 'Confirmar' -or $names -contains 'Confirm') -and ($names -match 'Remover|Remove')){return $parent}
        $parent=$walker.GetParent($parent);$depth++
      }
    }
    Start-Sleep -Milliseconds 150
  }
  throw 'Confirmation dialog scope could not be isolated safely'
}
function Wait-InitialCancelFocus($dialog){
  $until=[DateTime]::UtcNow.AddSeconds(3)
  while([DateTime]::UtcNow -lt $until){$cancel=Find-Ui $dialog ([System.Windows.Automation.ControlType]::Button) 'Cancelar';if(-not $cancel){$cancel=Find-Ui $dialog ([System.Windows.Automation.ControlType]::Button) 'Cancel'};if($cancel -and $cancel.Current.HasKeyboardFocus){return $cancel};Start-Sleep -Milliseconds 100}
  return $null
}
function Capture-UiRegion($el,[string]$path,$window){
 Need ($window.Current.IsOffscreen -eq $false) 'App window is offscreen';$hwnd=[IntPtr]$window.Current.NativeWindowHandle
 [void][LcCapture]::SetForegroundWindow($hwnd);Start-Sleep -Milliseconds 200
 Need ([LcCapture]::GetForegroundWindow() -eq $hwnd) 'App is not foreground; refusing screenshot'
 $r=$el.Current.BoundingRectangle;$wr=$window.Current.BoundingRectangle
 Need ($r.Width -gt 0 -and $r.Height -gt 0 -and $r.X -ge $wr.X -and $r.Y -ge $wr.Y -and ($r.X+$r.Width) -le ($wr.X+$wr.Width) -and ($r.Y+$r.Height) -le ($wr.Y+$wr.Height)) 'UIA capture bounds outside app window'
 Need ($r.Width -lt 800 -and $r.Height -lt 500) 'UIA capture region too large; refusing possible QR/other content'
 $qr=@((Get-UiTree $window)|Where-Object{$_.Current.Name -match '(?i)QR code|pairing QR|código de pareamento'}|ForEach-Object{$_.Current.BoundingRectangle})
 foreach($q in $qr){$overlap=($r.X -lt ($q.X+$q.Width) -and ($r.X+$r.Width) -gt $q.X -and $r.Y -lt ($q.Y+$q.Height) -and ($r.Y+$r.Height) -gt $q.Y);Need (-not $overlap) 'Capture region overlaps QR-labelled control'}
 $b=[Drawing.Bitmap]::new([int]$r.Width,[int]$r.Height);$g=[Drawing.Graphics]::FromImage($b);$g.CopyFromScreen([int]$r.X,[int]$r.Y,0,0,$b.Size);$g.Dispose();$b.Save($path,[Drawing.Imaging.ImageFormat]::Png);$b.Dispose()
}
function Remove-OwnedDevice($window,$scope,[string]$deviceName){
 $labels=@("Remover $deviceName","Remove $deviceName")
 foreach($label in $labels){$button=Find-Ui $scope ([System.Windows.Automation.ControlType]::Button) $label;if($button){Invoke-Ui $button;$dialog=Find-ConfirmDialog $scope;$confirm=Find-Ui $dialog ([System.Windows.Automation.ControlType]::Button) 'Confirmar';if(-not $confirm){$confirm=Find-Ui $dialog ([System.Windows.Automation.ControlType]::Button) 'Confirm'};if($confirm){Invoke-Ui $confirm;return (Wait-Name $scope $deviceName $false 10)}}}
 return $false
}
Add-Type @'
using System; using System.Runtime.InteropServices; public static class LcCapture {
 [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
 [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow(); }
'@
Add-Type @'
using System.Security.Cryptography;
public static class LcFixed { public static bool Equal(byte[] a, byte[] b) => a != null && b != null && a.Length == b.Length && CryptographicOperations.FixedTimeEquals(a,b); }
'@
try {
  $script:startedAt=[DateTime]::UtcNow
  foreach($file in @('F3-T06-AT-3.03-trusted.png','F3-T06-AT-3.03-confirm.png','F3-T06-AT-3.03-revoked.png','F3-T06-AT-3.03-approved.json','F3-T06-AT-3.03-attempt-3.json')){Need (-not (Test-Path (Join-Path $EvidenceDir $file))) 'Evidence destination already exists; refusing overwrite'}
 Need (Test-Path $MockExe) 'mock-sender binary missing; build not run by harness';$w=Get-Window;Need ($null -ne $w) 'LocalCamera window missing for supplied PID'
  $script:stage='connection-tab';$tab=Find-Ui $w ([System.Windows.Automation.ControlType]::TabItem) 'Conexão';if(-not $tab){$tab=Find-Ui $w ([System.Windows.Automation.ControlType]::TabItem) 'Connection'}
  if($tab){$tab.GetCurrentPattern([System.Windows.Automation.SelectionItemPattern]::Pattern).Select()}else{$tab=Find-Ui $w ([System.Windows.Automation.ControlType]::Button) 'Conexão';if(-not $tab){$tab=Find-Ui $w ([System.Windows.Automation.ControlType]::Button) 'Connection'};if($tab){Invoke-Ui $tab}else{Need ((Find-Ui $w ([System.Windows.Automation.ControlType]::Text) 'Dispositivos confiáveis') -or (Find-Ui $w ([System.Windows.Automation.ControlType]::Text) 'Trusted devices')) 'No connection tab or trusted heading in current UI'}}
  $script:stage='trusted-section';$until=[DateTime]::UtcNow.AddSeconds(12);$section=$null
  while([DateTime]::UtcNow -lt $until -and -not $section){$section=Find-Ui $w ([System.Windows.Automation.ControlType]::Group) 'Dispositivos confiáveis';if(-not $section){$section=Find-Ui $w ([System.Windows.Automation.ControlType]::Group) 'Trusted devices'};if(-not $section){Start-Sleep -Milliseconds 200}}
  Need ($section) 'Trusted section group unavailable';
  $script:stage='cdp-uri';$uri=Get-CdpUri
  $script:stage='pair-mock';$first=Start-Mock @('--uri',$uri,'--state-file',$state,'--name',$name,'--seconds','3');$script:firstPid=$first.Id
 $scope=$w; $approval=$null;$deadline=[DateTime]::UtcNow.AddSeconds(20)
  while([DateTime]::UtcNow -lt $deadline -and -not $approval){
    if($first.HasExited){$earlyCode=Wait-Exit $first 1;$script:stage='pair-mock';throw 'Pairing mock exited before approval'}
   $approval=Find-Ui $scope ([System.Windows.Automation.ControlType]::Button) 'Permitir sempre';if(-not $approval){$approval=Find-Ui $scope ([System.Windows.Automation.ControlType]::Button) 'Always allow'};if(-not $approval){Start-Sleep -Milliseconds 200}
  }
  $script:stage='approve-permanent';Need ($null -ne $approval) 'Permanent approval UIA button did not appear';Invoke-Ui $approval;$script:createdPermanent=$true
 $firstCode=Wait-Exit $first 30;Need ($firstCode -eq 0) 'Initial mock pairing failed'
 Need (Wait-Name $scope $name $true 20) 'Trusted device row not found';$matches=@((Get-UiTree $scope)|Where-Object{$_.Current.Name -eq $name});Need ($matches.Count -eq 1) 'Trusted row name is not unique'
 $section=Find-Ui $scope ([System.Windows.Automation.ControlType]::Group) 'Dispositivos confiáveis';if(-not $section){$section=Find-Ui $scope ([System.Windows.Automation.ControlType]::Group) 'Trusted devices'}
 Need ($section) 'Trusted section UIA group unavailable for safe capture';Capture-UiRegion $section (Join-Path $EvidenceDir 'F3-T06-AT-3.03-trusted.png') $w
 $removeName="Remover $name";$remove=Find-Ui $scope ([System.Windows.Automation.ControlType]::Button) $removeName;if(-not $remove){$remove=Find-Ui $scope ([System.Windows.Automation.ControlType]::Button) "Remove $name"};Invoke-Ui $remove
  $script:stage='verify-initial-focus';$dialog=Find-ConfirmDialog $scope;$cancel=Wait-InitialCancelFocus $dialog
  Need ($cancel) 'Cancel did not receive initial keyboard focus within 3s'
 Capture-UiRegion $dialog (Join-Path $EvidenceDir 'F3-T06-AT-3.03-confirm.png') $w;Invoke-Ui $cancel;Need (Wait-Name $scope $name $true 5) 'Cancel removed device'
 $remove=Find-Ui $scope ([System.Windows.Automation.ControlType]::Button) $removeName;if(-not $remove){$remove=Find-Ui $scope ([System.Windows.Automation.ControlType]::Button) "Remove $name"};Invoke-Ui $remove
 $dialog=Find-ConfirmDialog $scope
 $confirm=Find-Ui $dialog ([System.Windows.Automation.ControlType]::Button) 'Confirmar';if(-not $confirm){$confirm=Find-Ui $dialog ([System.Windows.Automation.ControlType]::Button) 'Confirm'};Invoke-Ui $confirm
  Need (Wait-Name $scope $name $false 12) 'Trusted row did not disappear after confirmation';$script:createdPermanent=$false
  $script:stage='reconnect-revoked';Need (Test-Path $state) 'New mock state file missing'
  $beforeBytes=[IO.File]::ReadAllBytes($state);$beforeHash=[Security.Cryptography.SHA256]::Create().ComputeHash($beforeBytes);[Array]::Clear($beforeBytes,0,$beforeBytes.Length)
 $hostPort=[int](([regex]::Match($uri,'[?&]port=(\d+)')).Groups[1].Value);Need ($hostPort -gt 0) 'Pairing URI omitted port';$uri=$null
 $second=Start-Mock @('--host','127.0.0.1','--port',[string]$hostPort,'--state-file',$state);$code=Wait-Exit $second 20
  $output=$script:outputs[$second.Id];$secondLines=$output.Out+"`n"+$output.Err;$revoked=($secondLines -match '(?i)revoked');$secondLines=$null
 Need ($code -eq 2) 'Revoked reconnection did not exit 2'
 Need $revoked 'Mock output did not report revoked reason'
 Need (-not (Wait-Name $scope $name $true 3)) 'Revoked device reappeared in trusted list'
  $script:stage='verify-state-identity';$afterBytes=[IO.File]::ReadAllBytes($state);$afterHash=[Security.Cryptography.SHA256]::Create().ComputeHash($afterBytes);[Array]::Clear($afterBytes,0,$afterBytes.Length)
  $sameFile=[LcFixed]::Equal($beforeHash,$afterHash);[Array]::Clear($beforeHash,0,$beforeHash.Length);[Array]::Clear($afterHash,0,$afterHash.Length);Need $sameFile 'State file changed before reconnect'
  $section=Find-Ui $scope ([System.Windows.Automation.ControlType]::Group) 'Dispositivos confiáveis';if(-not $section){$section=Find-Ui $scope ([System.Windows.Automation.ControlType]::Group) 'Trusted devices'}
  Need ($section) 'Trusted section missing after revocation';Capture-UiRegion $section (Join-Path $EvidenceDir 'F3-T06-AT-3.03-revoked.png') $w
  $shotHashes=[ordered]@{};foreach($shot in @('F3-T06-AT-3.03-trusted.png','F3-T06-AT-3.03-confirm.png','F3-T06-AT-3.03-revoked.png')){$shotHashes[$shot]=(Get-FileHash (Join-Path $EvidenceDir $shot) -Algorithm SHA256).Hash}
  $safe=[ordered]@{test='AT-3.03';result='PASS';diagnosticOnly=$false;desktopPid=$DesktopPid;mockPairExitCode=$firstCode;reconnectExitCode=$code;revokedReasonObserved=$revoked;sameStateFileFingerprint=$sameFile;trustedNameAbsent=$true;cancelInitiallyFocused=$true;hostsNormalizedToLoopback=$true;elapsedSeconds=[math]::Round(([DateTime]::UtcNow-$script:startedAt).TotalSeconds,2);screenshotSha256=$shotHashes;credential='state file same before/after; contents withheld';uri='withheld'}
  $safe|ConvertTo-Json -Depth 5|Set-Content -Encoding utf8 (Join-Path $EvidenceDir 'F3-T06-AT-3.03-approved.json');Write-Output ($safe|ConvertTo-Json -Depth 5 -Compress)
} catch {
 $line=$_.InvocationInfo.ScriptLineNumber;if($_.ScriptStackTrace -match 'AT-3\.03\.ps1: line (\d+)'){$line=$Matches[1]};$type=$_.Exception.GetType().FullName
 if($script:createdPermanent){try{$script:cleanup=if(Remove-OwnedDevice $w $scope $name){'removed-by-UIA'}else{'agent-cleanup-required'}}catch{$script:cleanup='agent-cleanup-required'}}
 $failure=[ordered]@{test='AT-3.03';result='BLOCKED';stage=$script:stage;exceptionType=$type;line=$line;mockExitCode=$script:firstExitCode;mockExitReason=$script:firstExitReason;hostsNormalizedToLoopback=$script:hostsNormalized;mockInvocation=$script:mockInvocation;ownedTrustedDeviceCleanup=$script:cleanup;ownedDeviceName=$(if($script:cleanup -eq 'agent-cleanup-required'){$name}else{$null});uri='withheld'}
  try{$failure|ConvertTo-Json|Set-Content -Encoding utf8 (Join-Path $EvidenceDir 'F3-T06-AT-3.03-attempt-3.json')}catch{}
  foreach($shot in @('F3-T06-AT-3.03-trusted.png','F3-T06-AT-3.03-confirm.png','F3-T06-AT-3.03-revoked.png')){Remove-Item (Join-Path $EvidenceDir $shot) -Force -ErrorAction SilentlyContinue}
 [Console]::Error.WriteLine("AT-3.03 failed stage=$script:stage type=$type line=$line mockExitCode=$script:firstExitCode mockExitReason=$script:firstExitReason cleanup=$script:cleanup")
 exit 1
}
finally {foreach($p in $started){try{if(-not $p.HasExited){$p.Kill();$p.WaitForExit(3000)}}catch{}};Remove-Item $state -Force -ErrorAction SilentlyContinue;Remove-Item $run -Recurse -Force -ErrorAction SilentlyContinue}
