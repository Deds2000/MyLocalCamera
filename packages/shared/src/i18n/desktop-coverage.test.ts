import { describe, expect, it } from "vitest";
import en from "./en.json";
import ptBR from "./pt-BR.json";

type JsonObject = Record<string, unknown>;

/**
 * Cobertura do namespace `desktop.*` (UT-3.25, parte automatizada) —
 * espelho estático de todas as seções `desktop` de `docs/ui/copy.md`
 * (fonte; 5 tabelas: conexão/vazio/aprovação, preview e barra, titlebar,
 * câmera/áudio/ajustes, saúde/assistente/erros). O `**negrito**` da fonte
 * é ênfase do Markdown do documento, não texto literal.
 * `desktop.smoke.*` não está aqui: não pertence ao `copy.md` (tela F0
 * removida na F3-T01) — ver Observações do PROGRESS.md.
 */
const DESKTOP_SOURCE: Record<string, { pt: string; en: string }> = {
  // desktop — conexão, vazio e aprovação
  "desktop.tabs.connection": { pt: "Conexão", en: "Connection" },
  "desktop.tabs.camera": { pt: "Câmera", en: "Camera" },
  "desktop.tabs.audio": { pt: "Áudio", en: "Audio" },
  "desktop.tabs.settings": { pt: "Ajustes", en: "Settings" },
  "desktop.empty.title": { pt: "Conecte seu celular", en: "Connect your phone" },
  "desktop.empty.subtitle": {
    pt: "Escaneie o QR code com o app LocalCamera ou plugue o cabo USB.",
    en: "Scan the QR code with the LocalCamera app or plug in a USB cable.",
  },
  "desktop.empty.network": { pt: "Rede: {{ip}} · Porta {{port}}", en: "Network: {{ip}} · Port {{port}}" },
  "desktop.empty.usbHint": {
    pt: "Ative a depuração USB no celular e plugue o cabo.",
    en: "Turn on USB debugging on your phone and plug in the cable.",
  },
  "desktop.connection.pairTitle": { pt: "Parear novo aparelho", en: "Pair a new device" },
  "desktop.connection.regenerate": { pt: "Regenerar QR code", en: "Regenerate QR code" },
  "desktop.connection.regenerateConfirm": {
    pt: "Regenerar invalida o QR atual. Aparelhos já pareados continuam funcionando.",
    en: "Regenerating invalidates the current QR. Already paired devices keep working.",
  },
  "desktop.connection.code": { pt: "Código de pareamento", en: "Pairing code" },
  "desktop.connection.interface": { pt: "Interface de rede", en: "Network interface" },
  "desktop.connection.disconnect": { pt: "Desconectar", en: "Disconnect" },
  "desktop.connection.usbTitle": { pt: "Aparelhos USB", en: "USB devices" },
  "desktop.connection.usbNone": { pt: "Nenhum aparelho detectado.", en: "No device detected." },
  "desktop.connection.usbAuto": {
    pt: "Conectar automaticamente ao plugar",
    en: "Connect automatically when plugged in",
  },
  "desktop.connection.connect": { pt: "Conectar", en: "Connect" },
  "desktop.adb.ready": { pt: "Pronto", en: "Ready" },
  "desktop.adb.unauthorized": { pt: "Autorize no celular", en: "Authorize on your phone" },
  "desktop.adb.unauthorizedHelp": {
    pt: "Toque em \"Permitir\" na caixa de diálogo exibida no celular.",
    en: "Tap \"Allow\" in the dialog shown on your phone.",
  },
  "desktop.adb.offline": { pt: "Offline", en: "Offline" },
  "desktop.adb.missing": { pt: "ADB não encontrado", en: "ADB not found" },
  "desktop.adb.download": { pt: "Baixar platform-tools", en: "Download platform-tools" },
  "desktop.adb.downloadConsent": {
    pt: "Vamos baixar o platform-tools oficial do Google para {{path}}. Continuar?",
    en: "We'll download Google's official platform-tools to {{path}}. Continue?",
  },
  "desktop.trusted.title": { pt: "Dispositivos confiáveis", en: "Trusted devices" },
  "desktop.trusted.empty": { pt: "Nenhum aparelho pareado.", en: "No paired devices." },
  "desktop.trusted.lastSeen": { pt: "Visto {{when}}", en: "Last seen {{when}}" },
  "desktop.trusted.removeConfirm": {
    pt: "Remover \"{{name}}\"? Ele precisará ser pareado e aprovado de novo.",
    en: "Remove \"{{name}}\"? It will need to be paired and approved again.",
  },
  "desktop.approval.title": { pt: "Permitir que \"{{name}}\" se conecte?", en: "Allow \"{{name}}\" to connect?" },
  "desktop.approval.body": {
    pt: "Modelo {{model}} · IP {{ip}}. Só aprove se for o seu aparelho.",
    en: "Model {{model}} · IP {{ip}}. Only approve if it's your device.",
  },
  "desktop.approval.always": { pt: "Permitir sempre", en: "Always allow" },
  "desktop.approval.once": { pt: "Só desta vez", en: "Just this time" },
  "desktop.approval.reject": { pt: "Recusar", en: "Decline" },
  "desktop.approval.timer": { pt: "Expira em {{seconds}} s", en: "Expires in {{seconds}} s" },
  "desktop.approval.notification": { pt: "Um aparelho quer se conectar", en: "A device wants to connect" },
  // desktop — preview e barra de ferramentas
  "desktop.preview.connecting": { pt: "Conectando…", en: "Connecting…" },
  "desktop.preview.pendingTitle": { pt: "Aguardando sua aprovação", en: "Waiting for your approval" },
  "desktop.preview.pendingAction": { pt: "Mostrar aviso", en: "Show prompt" },
  "desktop.preview.reconnecting": { pt: "Reconectando… ({{stage}})", en: "Reconnecting… ({{stage}})" },
  "desktop.preview.stopWaiting": { pt: "Parar de esperar", en: "Stop waiting" },
  "desktop.preview.dropped": { pt: "{{count}} perdidos", en: "{{count}} dropped" },
  "desktop.preview.dualBack": { pt: "Traseira", en: "Back" },
  "desktop.preview.dualFront": { pt: "Frontal", en: "Front" },
  "desktop.toolbar.mirror": { pt: "Espelhar", en: "Mirror" },
  "desktop.toolbar.rotate": { pt: "Girar 90°", en: "Rotate 90°" },
  "desktop.toolbar.switchCamera": { pt: "Alternar câmera", en: "Switch camera" },
  "desktop.toolbar.swapPip": { pt: "Trocar principal e miniatura", en: "Swap main and thumbnail" },
  "desktop.toolbar.fullscreen": { pt: "Tela cheia do preview", en: "Preview fullscreen" },
  "desktop.toolbar.exitFullscreen": { pt: "Sair da tela cheia", en: "Exit fullscreen" },
  "desktop.toolbar.mute": { pt: "Silenciar microfone", en: "Mute microphone" },
  "desktop.toolbar.unmute": { pt: "Ativar microfone", en: "Unmute microphone" },
  "desktop.toolbar.applying": { pt: "Aplicando…", en: "Applying…" },
  // desktop — titlebar
  "desktop.titlebar.logoLocal": { pt: "Local", en: "Local" },
  "desktop.titlebar.logoCamera": { pt: "Camera", en: "Camera" },
  "desktop.window.minimize": { pt: "Minimizar", en: "Minimize" },
  "desktop.window.maximize": { pt: "Maximizar", en: "Maximize" },
  "desktop.window.restore": { pt: "Restaurar", en: "Restore" },
  "desktop.window.close": { pt: "Fechar", en: "Close" },
  // desktop — câmera, áudio e ajustes
  "desktop.camera.source": { pt: "Câmera", en: "Camera" },
  "desktop.camera.dualUnavailable": {
    pt: "Seu aparelho não permite usar as duas câmeras ao mesmo tempo.",
    en: "Your device can't use both cameras at the same time.",
  },
  "desktop.camera.dualOutput": { pt: "Saída no modo dual", en: "Dual mode output" },
  "desktop.camera.dualSeparate": { pt: "Duas webcams", en: "Two webcams" },
  "desktop.camera.dualPip": { pt: "Uma webcam (PiP)", en: "One webcam (PiP)" },
  "desktop.camera.dualModeWarning": {
    pt: "Apps que estão usando a câmera precisarão reabri-la.",
    en: "Apps using the camera will need to reopen it.",
  },
  "desktop.camera.pipPosition": { pt: "Posição da miniatura", en: "Thumbnail position" },
  "desktop.camera.pipSize": { pt: "Tamanho da miniatura", en: "Thumbnail size" },
  "desktop.camera.swap": { pt: "Trocar principal e miniatura", en: "Swap main and thumbnail" },
  "desktop.camera.quality": { pt: "Qualidade", en: "Quality" },
  "desktop.camera.resolution": { pt: "Resolução", en: "Resolution" },
  "desktop.camera.fps": { pt: "Taxa de quadros", en: "Frame rate" },
  "desktop.camera.bitrateAuto": { pt: "Bitrate automático", en: "Automatic bitrate" },
  "desktop.camera.bitrate": { pt: "Bitrate", en: "Bitrate" },
  "desktop.camera.lowered": {
    pt: "Reduzido para {{value}} neste modo.",
    en: "Lowered to {{value}} in this mode.",
  },
  "desktop.camera.image": { pt: "Imagem", en: "Image" },
  "desktop.camera.mirror": { pt: "Espelhar", en: "Mirror" },
  "desktop.camera.rotation": { pt: "Rotação", en: "Rotation" },
  "desktop.camera.fit": { pt: "Ajuste da saída", en: "Output fit" },
  "desktop.camera.fitContain": { pt: "Conter", en: "Contain" },
  "desktop.camera.fitCover": { pt: "Preencher", en: "Fill" },
  "desktop.camera.appliedOnPhone": { pt: "Aplicado no celular", en: "Applied on the phone" },
  "desktop.camera.appliedOnPc": { pt: "Aplicado no PC", en: "Applied on the PC" },
  "desktop.audio.enabled": { pt: "Microfone do celular", en: "Phone microphone" },
  "desktop.audio.level": { pt: "Nível", en: "Level" },
  "desktop.audio.gain": { pt: "Ganho", en: "Gain" },
  "desktop.audio.gainWarn": { pt: "Pode distorcer.", en: "May distort." },
  "desktop.audio.delay": { pt: "Atraso do áudio", en: "Audio delay" },
  "desktop.audio.delayHint": {
    pt: "Aumente se o som chegar antes da imagem.",
    en: "Increase it if the sound arrives before the picture.",
  },
  "desktop.audio.output": { pt: "Saída (modo interino)", en: "Output (interim mode)" },
  "desktop.settings.language": { pt: "Idioma", en: "Language" },
  "desktop.settings.languageSystem": { pt: "Sistema", en: "System" },
  "desktop.settings.languagePortuguese": { pt: "Português", en: "Português" },
  "desktop.settings.languageEnglish": { pt: "English", en: "English" },
  "desktop.settings.startWithWindows": { pt: "Iniciar com o Windows", en: "Start with Windows" },
  "desktop.settings.trayOnClose": { pt: "Minimizar para a bandeja ao fechar", en: "Minimize to tray on close" },
  "desktop.settings.port": { pt: "Porta", en: "Port" },
  "desktop.settings.portRestart": {
    pt: "Reinicie o servidor para aplicar a nova porta.",
    en: "Restart the server to apply the new port.",
  },
  "desktop.settings.firewall": { pt: "Regra de firewall", en: "Firewall rule" },
  "desktop.settings.adbPath": { pt: "Caminho do ADB", en: "ADB path" },
  "desktop.settings.browse": { pt: "Procurar…", en: "Browse…" },
  "desktop.settings.vcam": { pt: "Câmera virtual", en: "Virtual camera" },
  "desktop.settings.repair": { pt: "Reparar", en: "Repair" },
  "desktop.settings.logLevel": { pt: "Nível de log", en: "Log level" },
  "desktop.settings.openLogs": { pt: "Abrir pasta de logs", en: "Open logs folder" },
  "desktop.settings.exportDiagnostics": { pt: "Exportar diagnóstico", en: "Export diagnostics" },
  "desktop.settings.about": { pt: "Sobre", en: "About" },
  // desktop — saúde, assistente e erros
  "desktop.health.server.ok": { pt: "Servidor :{{port}}", en: "Server :{{port}}" },
  "desktop.health.server.altPort": { pt: ":{{port}} (porta alterada)", en: ":{{port}} (port changed)" },
  "desktop.health.server.down": { pt: "Servidor parado", en: "Server stopped" },
  "desktop.health.discovery.ok": { pt: "Visível na rede", en: "Visible on the network" },
  "desktop.health.discovery.off": { pt: "Descoberta indisponível", en: "Discovery unavailable" },
  "desktop.health.vcam.notRegistered": { pt: "Câmera não registrada", en: "Camera not registered" },
  "desktop.health.vcam.ready": { pt: "Câmera pronta", en: "Camera ready" },
  "desktop.health.vcam.inUse": { pt: "Câmera em uso ({{count}})", en: "Camera in use ({{count}})" },
  "desktop.health.vcam.error": { pt: "Erro na câmera: {{code}}", en: "Camera error: {{code}}" },
  "desktop.health.mic.notInstalled": { pt: "Microfone não instalado", en: "Microphone not installed" },
  "desktop.health.mic.testMode": { pt: "Ative o modo de teste", en: "Turn on test mode" },
  "desktop.health.mic.blocked": { pt: "Bloqueado pelo Windows", en: "Blocked by Windows" },
  "desktop.health.mic.ready": { pt: "Microfone pronto", en: "Microphone ready" },
  "desktop.health.adb.ok": { pt: "ADB v{{version}}", en: "ADB v{{version}}" },
  "desktop.health.adb.missing": { pt: "ADB ausente", en: "ADB missing" },
  "desktop.health.soon": { pt: "Em breve", en: "Coming soon" },
  "desktop.wizard.title": { pt: "Vamos deixar tudo pronto", en: "Let's get everything ready" },
  "desktop.wizard.step.network": { pt: "Rede", en: "Network" },
  "desktop.wizard.step.camera": { pt: "Câmera virtual", en: "Virtual camera" },
  "desktop.wizard.step.mic": { pt: "Microfone virtual (opcional)", en: "Virtual microphone (optional)" },
  "desktop.wizard.step.adb": { pt: "ADB (opcional)", en: "ADB (optional)" },
  "desktop.wizard.skip": { pt: "Pular por enquanto", en: "Skip for now" },
  "desktop.wizard.done": {
    pt: "Tudo pronto. Escaneie o QR code com o app do celular.",
    en: "All set. Scan the QR code with the phone app.",
  },
  "desktop.vcam.notRegistered": {
    pt: "A câmera virtual ainda não foi registrada.",
    en: "The virtual camera hasn't been registered yet.",
  },
  "desktop.errors.portInUse": { pt: "A porta {{port}} está em uso. Usando {{alt}}.", en: "Port {{port}} is in use. Using {{alt}}." },
  "desktop.errors.firewall": { pt: "O firewall pode estar bloqueando as conexões.", en: "The firewall may be blocking connections." },
  "desktop.errors.decode": { pt: "Imagem retomada após um erro.", en: "Video resumed after an error." },
  "desktop.toast.deviceConnected": { pt: "{{name}} conectado", en: "{{name}} connected" },
  "desktop.toast.deviceDisconnected": { pt: "{{name}} desconectado", en: "{{name}} disconnected" },
  "desktop.toast.settingRejected": { pt: "O celular não aceitou essa configuração.", en: "The phone didn't accept that setting." },
};

function leafAt(root: JsonObject, path: string): unknown {
  let node: unknown = root;
  for (const part of path.split(".")) {
    if (node === null || typeof node !== "object") return undefined;
    node = (node as JsonObject)[part];
  }
  return node;
}

function variables(text: string): string[] {
  const found = text.match(/\{\{(\w+)\}\}/g) ?? [];
  return found.map((v) => v.slice(2, -2)).sort();
}

describe("F3-T14b cobertura desktop.* (fonte: docs/ui/copy.md)", () => {
  it("a lista estática espelha as seções desktop do copy.md (136 chaves)", () => {
    expect(Object.keys(DESKTOP_SOURCE).length).toBe(136);
  });

  it("toda chave desktop existe como string em pt-BR e em en", () => {
    for (const key of Object.keys(DESKTOP_SOURCE)) {
      expect(leafAt(ptBR as JsonObject, key), `pt-BR: ${key}`).toBeTypeOf("string");
      expect(leafAt(en as JsonObject, key), `en: ${key}`).toBeTypeOf("string");
    }
  });

  it("os valores batem com a fonte do copy.md", () => {
    for (const [key, source] of Object.entries(DESKTOP_SOURCE)) {
      expect(leafAt(ptBR as JsonObject, key), `pt-BR: ${key}`).toBe(source.pt);
      expect(leafAt(en as JsonObject, key), `en: ${key}`).toBe(source.en);
    }
  });

  it("as variáveis de interpolação batem entre pt-BR e en", () => {
    for (const key of Object.keys(DESKTOP_SOURCE)) {
      const ptValue = leafAt(ptBR as JsonObject, key) as string;
      const enValue = leafAt(en as JsonObject, key) as string;
      expect(variables(ptValue), `interpolação pt-BR: ${key}`).toEqual(variables(enValue));
    }
  });
});
