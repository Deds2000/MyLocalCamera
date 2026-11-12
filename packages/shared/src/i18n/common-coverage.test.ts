import { describe, expect, it } from "vitest";
import en from "./en.json";
import ptBR from "./pt-BR.json";

type JsonObject = Record<string, unknown>;

/**
 * Cobertura do namespace `common.*` — espelho da tabela da seção `common`
 * de `docs/ui/copy.md` (fonte). O `**negrito**` da fonte é ênfase do
 * Markdown do documento, não texto literal; as entradas trazem o texto puro.
 */
const COMMON_SOURCE: Record<string, { pt: string; en: string }> = {
  "common.appName": { pt: "LocalCamera", en: "LocalCamera" },
  "common.actions.cancel": { pt: "Cancelar", en: "Cancel" },
  "common.actions.confirm": { pt: "Confirmar", en: "Confirm" },
  "common.actions.close": { pt: "Fechar", en: "Close" },
  "common.actions.copy": { pt: "Copiar", en: "Copy" },
  "common.actions.copied": { pt: "Copiado!", en: "Copied!" },
  "common.actions.retry": { pt: "Tentar de novo", en: "Try again" },
  "common.actions.fix": { pt: "Corrigir", en: "Fix" },
  "common.actions.remove": { pt: "Remover", en: "Remove" },
  "common.actions.back": { pt: "Voltar", en: "Back" },
  "common.actions.openSettings": { pt: "Abrir configurações", en: "Open settings" },
  "common.state.idle": { pt: "Desconectado", en: "Disconnected" },
  "common.state.connecting": { pt: "Conectando…", en: "Connecting…" },
  "common.state.pending": { pt: "Aguardando aprovação…", en: "Waiting for approval…" },
  "common.state.streaming": { pt: "Transmitindo", en: "Streaming" },
  "common.state.reconnecting": { pt: "Reconectando…", en: "Reconnecting…" },
  "common.state.live": { pt: "AO VIVO", en: "LIVE" },
  "common.camera.back": { pt: "Traseira", en: "Back" },
  "common.camera.front": { pt: "Frontal", en: "Front" },
  "common.camera.dual": { pt: "Dupla", en: "Dual" },
  "common.transport.wifi": { pt: "Wi-Fi", en: "Wi-Fi" },
  "common.transport.usb": { pt: "USB", en: "USB" },
  "common.units.mbps": { pt: "{{value}} Mbps", en: "{{value}} Mbps" },
  "common.units.ms": { pt: "{{value}} ms", en: "{{value}} ms" },
  "common.units.fps": { pt: "{{value}} fps", en: "{{value}} fps" },
  "common.errors.bad_token": {
    pt: "Código de pareamento inválido. Escaneie o QR code de novo.",
    en: "Invalid pairing code. Scan the QR code again.",
  },
  "common.errors.not_approved": {
    pt: "A conexão não foi aprovada no PC.",
    en: "The connection wasn't approved on the PC.",
  },
  "common.errors.revoked": {
    pt: "Este aparelho foi removido do PC. Pareie de novo com o QR code.",
    en: "This device was removed from the PC. Pair again with the QR code.",
  },
  "common.errors.busy": {
    pt: "O PC já está conectado a outro celular.",
    en: "The PC is already connected to another phone.",
  },
  "common.errors.version_mismatch.phone": {
    pt: "Atualize o app do celular (versão {{phone}}; o PC usa {{pc}}).",
    en: "Update the phone app (version {{phone}}; the PC uses {{pc}}).",
  },
  "common.errors.version_mismatch.pc": {
    pt: "Atualize o app do PC (versão {{pc}}; o celular usa {{phone}}).",
    en: "Update the PC app (version {{pc}}; the phone uses {{phone}}).",
  },
  "common.errors.generic": {
    pt: "Falha na comunicação. Tentando reconectar.",
    en: "Communication failed. Trying to reconnect.",
  },
  "common.errors.noNetwork": {
    pt: "Sem rede. Conecte-se ao Wi-Fi ou use o cabo USB.",
    en: "No network. Connect to Wi-Fi or use a USB cable.",
  },
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

describe("F3-T03 cobertura common.* (fonte: docs/ui/copy.md)", () => {
  it("a lista estática espelha a seção common do copy.md (33 chaves)", () => {
    expect(Object.keys(COMMON_SOURCE).length).toBe(33);
  });

  it("toda chave comum existe como string em pt-BR e em en", () => {
    for (const key of Object.keys(COMMON_SOURCE)) {
      expect(leafAt(ptBR as JsonObject, key), `pt-BR: ${key}`).toBeTypeOf("string");
      expect(leafAt(en as JsonObject, key), `en: ${key}`).toBeTypeOf("string");
    }
  });

  it("os valores batem com a fonte do copy.md", () => {
    for (const [key, source] of Object.entries(COMMON_SOURCE)) {
      expect(leafAt(ptBR as JsonObject, key), `pt-BR: ${key}`).toBe(source.pt);
      expect(leafAt(en as JsonObject, key), `en: ${key}`).toBe(source.en);
    }
  });

  it("as variáveis de interpolação batem entre pt-BR e en", () => {
    for (const key of Object.keys(COMMON_SOURCE)) {
      const ptValue = leafAt(ptBR as JsonObject, key) as string;
      const enValue = leafAt(en as JsonObject, key) as string;
      expect(variables(ptValue), `interpolação pt-BR: ${key}`).toEqual(variables(enValue));
    }
  });
});
