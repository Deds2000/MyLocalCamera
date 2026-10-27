# Textos da interface (pt-BR e en)

Fonte inicial para `packages/shared/src/i18n/{pt-BR,en}.json`. Interpolação no formato do i18next (`{{variavel}}`); plurais com sufixos `_one`/`_other`. **Tom:** direto, calmo, sem jargão; segunda pessoa informal ("você"). Chaves com `.` viram objetos aninhados nos JSON.

Regras: nenhuma frase vem da rede (só **códigos**); textos em pt-BR podem ser ~30% mais longos que em inglês — os layouts devem suportar isso.

---

## common

| Chave | pt-BR | en |
|---|---|---|
| `common.appName` | LocalCamera | LocalCamera |
| `common.actions.cancel` | Cancelar | Cancel |
| `common.actions.confirm` | Confirmar | Confirm |
| `common.actions.close` | Fechar | Close |
| `common.actions.copy` | Copiar | Copy |
| `common.actions.copied` | Copiado! | Copied! |
| `common.actions.retry` | Tentar de novo | Try again |
| `common.actions.fix` | Corrigir | Fix |
| `common.actions.remove` | Remover | Remove |
| `common.actions.back` | Voltar | Back |
| `common.actions.openSettings` | Abrir configurações | Open settings |
| `common.state.idle` | Desconectado | Disconnected |
| `common.state.connecting` | Conectando… | Connecting… |
| `common.state.pending` | Aguardando aprovação… | Waiting for approval… |
| `common.state.streaming` | Transmitindo | Streaming |
| `common.state.reconnecting` | Reconectando… | Reconnecting… |
| `common.state.live` | AO VIVO | LIVE |
| `common.camera.back` | Traseira | Back |
| `common.camera.front` | Frontal | Front |
| `common.camera.dual` | Dupla | Dual |
| `common.transport.wifi` | Wi-Fi | Wi-Fi |
| `common.transport.usb` | USB | USB |
| `common.units.mbps` | {{value}} Mbps | {{value}} Mbps |
| `common.units.ms` | {{value}} ms | {{value}} ms |
| `common.units.fps` | {{value}} fps | {{value}} fps |
| `common.errors.bad_token` | Código de pareamento inválido. Escaneie o QR code de novo. | Invalid pairing code. Scan the QR code again. |
| `common.errors.not_approved` | A conexão não foi aprovada no PC. | The connection wasn't approved on the PC. |
| `common.errors.revoked` | Este aparelho foi removido do PC. Pareie de novo com o QR code. | This device was removed from the PC. Pair again with the QR code. |
| `common.errors.busy` | O PC já está conectado a outro celular. | The PC is already connected to another phone. |
| `common.errors.version_mismatch.phone` | Atualize o app do **celular** (versão {{phone}}; o PC usa {{pc}}). | Update the **phone** app (version {{phone}}; the PC uses {{pc}}). |
| `common.errors.version_mismatch.pc` | Atualize o app do **PC** (versão {{pc}}; o celular usa {{phone}}). | Update the **PC** app (version {{pc}}; the phone uses {{phone}}). |
| `common.errors.generic` | Falha na comunicação. Tentando reconectar. | Communication failed. Trying to reconnect. |
| `common.errors.noNetwork` | Sem rede. Conecte-se ao Wi-Fi ou use o cabo USB. | No network. Connect to Wi-Fi or use a USB cable. |

## desktop — conexão, vazio e aprovação

| Chave | pt-BR | en |
|---|---|---|
| `desktop.tabs.connection` | Conexão | Connection |
| `desktop.tabs.camera` | Câmera | Camera |
| `desktop.tabs.audio` | Áudio | Audio |
| `desktop.tabs.settings` | Ajustes | Settings |
| `desktop.empty.title` | Conecte seu celular | Connect your phone |
| `desktop.empty.subtitle` | Escaneie o QR code com o app LocalCamera ou plugue o cabo USB. | Scan the QR code with the LocalCamera app or plug in a USB cable. |
| `desktop.empty.network` | Rede: {{ip}} · Porta {{port}} | Network: {{ip}} · Port {{port}} |
| `desktop.empty.usbHint` | Ative a depuração USB no celular e plugue o cabo. | Turn on USB debugging on your phone and plug in the cable. |
| `desktop.connection.pairTitle` | Parear novo aparelho | Pair a new device |
| `desktop.connection.regenerate` | Regenerar QR code | Regenerate QR code |
| `desktop.connection.regenerateConfirm` | Regenerar invalida o QR atual. Aparelhos já pareados continuam funcionando. | Regenerating invalidates the current QR. Already paired devices keep working. |
| `desktop.connection.code` | Código de pareamento | Pairing code |
| `desktop.connection.interface` | Interface de rede | Network interface |
| `desktop.connection.disconnect` | Desconectar | Disconnect |
| `desktop.connection.usbTitle` | Aparelhos USB | USB devices |
| `desktop.connection.usbNone` | Nenhum aparelho detectado. | No device detected. |
| `desktop.connection.usbAuto` | Conectar automaticamente ao plugar | Connect automatically when plugged in |
| `desktop.connection.connect` | Conectar | Connect |
| `desktop.adb.ready` | Pronto | Ready |
| `desktop.adb.unauthorized` | Autorize no celular | Authorize on your phone |
| `desktop.adb.unauthorizedHelp` | Toque em "Permitir" na caixa de diálogo exibida no celular. | Tap "Allow" in the dialog shown on your phone. |
| `desktop.adb.offline` | Offline | Offline |
| `desktop.adb.missing` | ADB não encontrado | ADB not found |
| `desktop.adb.download` | Baixar platform-tools | Download platform-tools |
| `desktop.adb.downloadConsent` | Vamos baixar o platform-tools oficial do Google para {{path}}. Continuar? | We'll download Google's official platform-tools to {{path}}. Continue? |
| `desktop.trusted.title` | Dispositivos confiáveis | Trusted devices |
| `desktop.trusted.empty` | Nenhum aparelho pareado. | No paired devices. |
| `desktop.trusted.lastSeen` | Visto {{when}} | Last seen {{when}} |
| `desktop.trusted.removeConfirm` | Remover "{{name}}"? Ele precisará ser pareado e aprovado de novo. | Remove "{{name}}"? It will need to be paired and approved again. |
| `desktop.approval.title` | Permitir que "{{name}}" se conecte? | Allow "{{name}}" to connect? |
| `desktop.approval.body` | Modelo {{model}} · IP {{ip}}. Só aprove se for o seu aparelho. | Model {{model}} · IP {{ip}}. Only approve if it's your device. |
| `desktop.approval.always` | Permitir sempre | Always allow |
| `desktop.approval.once` | Só desta vez | Just this time |
| `desktop.approval.reject` | Recusar | Decline |
| `desktop.approval.timer` | Expira em {{seconds}} s | Expires in {{seconds}} s |
| `desktop.approval.notification` | Um aparelho quer se conectar | A device wants to connect |

## desktop — preview e barra de ferramentas

| Chave | pt-BR | en |
|---|---|---|
| `desktop.preview.connecting` | Conectando… | Connecting… |
| `desktop.preview.pendingTitle` | Aguardando sua aprovação | Waiting for your approval |
| `desktop.preview.pendingAction` | Mostrar aviso | Show prompt |
| `desktop.preview.reconnecting` | Reconectando… ({{stage}}) | Reconnecting… ({{stage}}) |
| `desktop.preview.stopWaiting` | Parar de esperar | Stop waiting |
| `desktop.preview.dropped` | {{count}} perdidos | {{count}} dropped |
| `desktop.preview.dualBack` | Traseira | Back |
| `desktop.preview.dualFront` | Frontal | Front |
| `desktop.toolbar.mirror` | Espelhar | Mirror |
| `desktop.toolbar.rotate` | Girar 90° | Rotate 90° |
| `desktop.toolbar.switchCamera` | Alternar câmera | Switch camera |
| `desktop.toolbar.swapPip` | Trocar principal e miniatura | Swap main and thumbnail |
| `desktop.toolbar.fullscreen` | Tela cheia do preview | Preview fullscreen |
| `desktop.toolbar.exitFullscreen` | Sair da tela cheia | Exit fullscreen |
| `desktop.toolbar.mute` | Silenciar microfone | Mute microphone |
| `desktop.toolbar.unmute` | Ativar microfone | Unmute microphone |
| `desktop.toolbar.applying` | Aplicando… | Applying… |

## desktop — câmera, áudio e ajustes

| Chave | pt-BR | en |
|---|---|---|
| `desktop.camera.source` | Câmera | Camera |
| `desktop.camera.dualUnavailable` | Seu aparelho não permite usar as duas câmeras ao mesmo tempo. | Your device can't use both cameras at the same time. |
| `desktop.camera.dualOutput` | Saída no modo dual | Dual mode output |
| `desktop.camera.dualSeparate` | Duas webcams | Two webcams |
| `desktop.camera.dualPip` | Uma webcam (PiP) | One webcam (PiP) |
| `desktop.camera.dualModeWarning` | Apps que estão usando a câmera precisarão reabri-la. | Apps using the camera will need to reopen it. |
| `desktop.camera.pipPosition` | Posição da miniatura | Thumbnail position |
| `desktop.camera.pipSize` | Tamanho da miniatura | Thumbnail size |
| `desktop.camera.swap` | Trocar principal e miniatura | Swap main and thumbnail |
| `desktop.camera.quality` | Qualidade | Quality |
| `desktop.camera.resolution` | Resolução | Resolution |
| `desktop.camera.fps` | Taxa de quadros | Frame rate |
| `desktop.camera.bitrateAuto` | Bitrate automático | Automatic bitrate |
| `desktop.camera.bitrate` | Bitrate | Bitrate |
| `desktop.camera.lowered` | Reduzido para {{value}} neste modo. | Lowered to {{value}} in this mode. |
| `desktop.camera.image` | Imagem | Image |
| `desktop.camera.mirror` | Espelhar | Mirror |
| `desktop.camera.rotation` | Rotação | Rotation |
| `desktop.camera.fit` | Ajuste da saída | Output fit |
| `desktop.camera.fitContain` | Conter | Contain |
| `desktop.camera.fitCover` | Preencher | Fill |
| `desktop.camera.appliedOnPhone` | Aplicado no celular | Applied on the phone |
| `desktop.camera.appliedOnPc` | Aplicado no PC | Applied on the PC |
| `desktop.audio.enabled` | Microfone do celular | Phone microphone |
| `desktop.audio.level` | Nível | Level |
| `desktop.audio.gain` | Ganho | Gain |
| `desktop.audio.gainWarn` | Pode distorcer. | May distort. |
| `desktop.audio.delay` | Atraso do áudio | Audio delay |
| `desktop.audio.delayHint` | Aumente se o som chegar antes da imagem. | Increase it if the sound arrives before the picture. |
| `desktop.audio.output` | Saída (modo interino) | Output (interim mode) |
| `desktop.settings.language` | Idioma | Language |
| `desktop.settings.languageSystem` | Sistema | System |
| `desktop.settings.startWithWindows` | Iniciar com o Windows | Start with Windows |
| `desktop.settings.trayOnClose` | Minimizar para a bandeja ao fechar | Minimize to tray on close |
| `desktop.settings.port` | Porta | Port |
| `desktop.settings.portRestart` | Reinicie o servidor para aplicar a nova porta. | Restart the server to apply the new port. |
| `desktop.settings.firewall` | Regra de firewall | Firewall rule |
| `desktop.settings.adbPath` | Caminho do ADB | ADB path |
| `desktop.settings.browse` | Procurar… | Browse… |
| `desktop.settings.vcam` | Câmera virtual | Virtual camera |
| `desktop.settings.repair` | Reparar | Repair |
| `desktop.settings.logLevel` | Nível de log | Log level |
| `desktop.settings.openLogs` | Abrir pasta de logs | Open logs folder |
| `desktop.settings.exportDiagnostics` | Exportar diagnóstico | Export diagnostics |
| `desktop.settings.about` | Sobre | About |

## desktop — saúde, assistente e erros

| Chave | pt-BR | en |
|---|---|---|
| `desktop.health.server.ok` | Servidor :{{port}} | Server :{{port}} |
| `desktop.health.server.altPort` | :{{port}} (porta alterada) | :{{port}} (port changed) |
| `desktop.health.server.down` | Servidor parado | Server stopped |
| `desktop.health.discovery.ok` | Visível na rede | Visible on the network |
| `desktop.health.discovery.off` | Descoberta indisponível | Discovery unavailable |
| `desktop.health.vcam.notRegistered` | Câmera não registrada | Camera not registered |
| `desktop.health.vcam.ready` | Câmera pronta | Camera ready |
| `desktop.health.vcam.inUse` | Câmera em uso ({{count}}) | Camera in use ({{count}}) |
| `desktop.health.vcam.error` | Erro na câmera: {{code}} | Camera error: {{code}} |
| `desktop.health.mic.notInstalled` | Microfone não instalado | Microphone not installed |
| `desktop.health.mic.testMode` | Ative o modo de teste | Turn on test mode |
| `desktop.health.mic.blocked` | Bloqueado pelo Windows | Blocked by Windows |
| `desktop.health.mic.ready` | Microfone pronto | Microphone ready |
| `desktop.health.adb.ok` | ADB v{{version}} | ADB v{{version}} |
| `desktop.health.adb.missing` | ADB ausente | ADB missing |
| `desktop.health.soon` | Em breve | Coming soon |
| `desktop.wizard.title` | Vamos deixar tudo pronto | Let's get everything ready |
| `desktop.wizard.step.network` | Rede | Network |
| `desktop.wizard.step.camera` | Câmera virtual | Virtual camera |
| `desktop.wizard.step.mic` | Microfone virtual (opcional) | Virtual microphone (optional) |
| `desktop.wizard.step.adb` | ADB (opcional) | ADB (optional) |
| `desktop.wizard.skip` | Pular por enquanto | Skip for now |
| `desktop.wizard.done` | Tudo pronto. Escaneie o QR code com o app do celular. | All set. Scan the QR code with the phone app. |
| `desktop.vcam.notRegistered` | A câmera virtual ainda não foi registrada. | The virtual camera hasn't been registered yet. |
| `desktop.errors.portInUse` | A porta {{port}} está em uso. Usando {{alt}}. | Port {{port}} is in use. Using {{alt}}. |
| `desktop.errors.firewall` | O firewall pode estar bloqueando as conexões. | The firewall may be blocking connections. |
| `desktop.errors.decode` | Imagem retomada após um erro. | Video resumed after an error. |
| `desktop.toast.deviceConnected` | {{name}} conectado | {{name}} connected |
| `desktop.toast.deviceDisconnected` | {{name}} desconectado | {{name}} disconnected |
| `desktop.toast.settingRejected` | O celular não aceitou essa configuração. | The phone didn't accept that setting. |

## mobile

| Chave | pt-BR | en |
|---|---|---|
| `mobile.permissions.title` | Precisamos de algumas permissões | We need a few permissions |
| `mobile.permissions.camera` | Câmera — para enviar a imagem ao PC. | Camera — to send video to your PC. |
| `mobile.permissions.mic` | Microfone — para enviar o áudio. | Microphone — to send audio. |
| `mobile.permissions.notifications` | Notificações — mantêm a transmissão ativa. | Notifications — keep the stream running. |
| `mobile.permissions.grant` | Permitir | Allow |
| `mobile.permissions.granted` | Concedida | Granted |
| `mobile.permissions.denied` | Permissão negada. Ative nas configurações do Android. | Permission denied. Turn it on in Android settings. |
| `mobile.permissions.continue` | Continuar | Continue |
| `mobile.permissions.cameraRequired` | A câmera é obrigatória para continuar. | The camera is required to continue. |
| `mobile.home.title` | Pronto para conectar | Ready to connect |
| `mobile.home.scan` | Escanear QR code | Scan QR code |
| `mobile.home.manual` | Inserir manualmente | Enter manually |
| `mobile.home.reconnect` | Reconectar a {{name}} | Reconnect to {{name}} |
| `mobile.home.seen` | visto {{when}} | seen {{when}} |
| `mobile.home.usbTip` | Prefere cabo? Plugue o USB e toque em Conectar no app do PC. | Prefer a cable? Plug in USB and click Connect in the PC app. |
| `mobile.home.knownPcs` | Meus PCs | My PCs |
| `mobile.scan.title` | Escanear QR code | Scan QR code |
| `mobile.scan.hint` | Aponte para o QR code exibido no PC | Point at the QR code shown on your PC |
| `mobile.scan.invalid` | Este QR code não é do LocalCamera. | This QR code isn't from LocalCamera. |
| `mobile.scan.torch` | Lanterna | Flashlight |
| `mobile.manual.title` | Inserir manualmente | Enter manually |
| `mobile.manual.address` | Endereço do PC | PC address |
| `mobile.manual.port` | Porta | Port |
| `mobile.manual.code` | Código de pareamento | Pairing code |
| `mobile.manual.paste` | Colar link | Paste link |
| `mobile.manual.connect` | Conectar | Connect |
| `mobile.manual.invalidAddress` | Endereço inválido. | Invalid address. |
| `mobile.manual.invalidCode` | O código tem 32 caracteres (0–9, a–f). | The code has 32 characters (0–9, a–f). |
| `mobile.connecting.title` | Conectando a {{name}}… | Connecting to {{name}}… |
| `mobile.connecting.failed` | Não foi possível conectar. | Couldn't connect. |
| `mobile.pending.title` | Aprove no PC | Approve on your PC |
| `mobile.pending.body` | Um aviso apareceu no computador. Toque em "Permitir sempre". | A prompt appeared on your computer. Click "Always allow". |
| `mobile.reconnect.title` | Reconectando | Reconnecting |
| `mobile.reconnect.discovery` | Procurando o PC na rede… | Looking for your PC on the network… |
| `mobile.reconnect.lastAddresses` | Tentando os últimos endereços… | Trying the last known addresses… |
| `mobile.reconnect.failed` | Não encontramos o PC. Escaneie o QR code. | We couldn't find your PC. Scan the QR code. |
| `mobile.stream.status` | Transmitindo para {{name}} | Streaming to {{name}} |
| `mobile.stream.bitrate` | Bitrate | Bitrate |
| `mobile.stream.latency` | Latência | Latency |
| `mobile.stream.dropped` | Quadros perdidos | Dropped frames |
| `mobile.stream.camera` | Câmera | Camera |
| `mobile.stream.dualBoth` | Traseira + Frontal | Back + Front |
| `mobile.stream.mic` | Microfone | Microphone |
| `mobile.stream.mirror` | Espelho | Mirror |
| `mobile.stream.rotate` | Girar | Rotate |
| `mobile.stream.stop` | Parar | Stop |
| `mobile.stream.stopConfirmTitle` | Parar a transmissão? | Stop streaming? |
| `mobile.stream.warnSlow` | Rede lenta — qualidade reduzida. | Slow network — quality lowered. |
| `mobile.stream.warnHot` | O aparelho está esquentando. | The device is getting warm. |
| `mobile.stream.warnBattery` | O sistema pode pausar a transmissão. | The system may pause the stream. |
| `mobile.knownPcs.title` | Meus PCs | My PCs |
| `mobile.knownPcs.empty` | Nenhum PC pareado ainda. | No paired PCs yet. |
| `mobile.knownPcs.forget` | Esquecer | Forget |
| `mobile.knownPcs.forgetConfirm` | Esquecer "{{name}}"? Será preciso escanear o QR e aprovar de novo. | Forget "{{name}}"? You'll need to scan the QR and approve again. |
| `mobile.error.badToken.title` | Código inválido | Invalid code |
| `mobile.error.notApproved.title` | Conexão não aprovada | Connection not approved |
| `mobile.error.revoked.title` | Este aparelho foi removido do PC | This device was removed from the PC |
| `mobile.error.busy.title` | PC ocupado | PC busy |
| `mobile.error.version.title` | Atualize o app | Update the app |
| `mobile.error.generic.title` | Falha na comunicação | Communication failed |
| `mobile.settings.title` | Configurações | Settings |
| `mobile.settings.transmission` | Transmissão | Streaming |
| `mobile.settings.background` | Continuar transmitindo com a tela apagada | Keep streaming with the screen off |
| `mobile.settings.backgroundHint` | Mantém a câmera ativa em segundo plano, com uma notificação. | Keeps the camera active in the background, with a notification. |
| `mobile.settings.keepScreenOn` | Manter a tela ligada | Keep the screen on |
| `mobile.settings.keepScreenOnHint` | A tela não apaga enquanto o app está aberto transmitindo. | The screen stays on while the app is open and streaming. |
| `mobile.settings.bothOff` | A transmissão para quando você sair do app. | Streaming stops when you leave the app. |
| `mobile.settings.defaultQuality` | Qualidade padrão | Default quality |
| `mobile.settings.micSource` | Fonte do microfone | Microphone source |
| `mobile.settings.micSourceMic` | Microfone | Microphone |
| `mobile.settings.micSourceVoice` | Comunicação de voz | Voice communication |
| `mobile.settings.language` | Idioma | Language |
| `mobile.settings.batteryHelp` | Otimização de bateria | Battery optimization |
| `mobile.settings.about` | Sobre | About |
| `mobile.battery.title` | Evite que o sistema pause a transmissão | Keep the system from pausing the stream |
| `mobile.battery.body` | Alguns fabricantes encerram apps em segundo plano para economizar bateria. Defina o LocalCamera como "Sem restrições". | Some manufacturers close background apps to save battery. Set LocalCamera to "Unrestricted". |
| `mobile.battery.open` | Abrir configurações de bateria | Open battery settings |

## Android nativo (`strings.xml`)

| Chave | `values-pt-rBR` | `values` (en) |
|---|---|---|
| `app_name` | LocalCamera | LocalCamera |
| `channel_streaming_name` | Transmissão | Streaming |
| `channel_streaming_desc` | Mostra quando o LocalCamera está transmitindo | Shows when LocalCamera is streaming |
| `notif_title_streaming` | LocalCamera — transmitindo | LocalCamera — streaming |
| `notif_text_streaming` | {PC} · {qualidade} · {transporte} | {PC} · {quality} · {transport} |
| `notif_text_reconnecting` | Reconectando… | Reconnecting… |
| `notif_action_stop` | Parar | Stop |
| `permission_rationale_camera` | Usamos a câmera para enviar a imagem ao seu PC. | We use the camera to send video to your PC. |
| `permission_rationale_mic` | Usamos o microfone para enviar o áudio ao seu PC. | We use the microphone to send audio to your PC. |
