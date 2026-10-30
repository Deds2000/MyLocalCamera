// F2-T03 — Isola o leitor de QR atrás de uma interface estável.
//
// Decisão (Fase 2): nenhum plugin de scanner foi instalado — nenhuma
// combinação verificada cobre Capacitor 8.5 + React 19 sem risco de API
// inventada, e a parte nativa (CameraX + ML Kit) pertence ao escopo Kotlin
// (F2-T07/F2-T13). A contingência do §9 da fase vale na forma mais leve:
// o scanner nativo futuro será injetado via prop `qrScanner` do
// `AppProvider`; até lá `getQrScanner()` devolve um stub
// (`available: false`) e o fallback funcional é a tela Manual
// (campo "Colar link" + campos manuais).

export interface QrScannerHandle {
  stop(): Promise<void>;
  /** Alterna a lanterna; devolve `false` se não suportado. */
  setTorch(on: boolean): Promise<boolean>;
}

export interface QrScannerApi {
  /** `false` enquanto o scanner nativo não existe (F2-T07/F2-T13). */
  readonly available: boolean;
  /** Inicia a leitura; `onCode` recebe o texto bruto do QR. */
  start(onCode: (raw: string) => void): Promise<QrScannerHandle>;
}

/** Implementação atual (stub). Substituir pela implementação nativa. */
export function getQrScanner(): QrScannerApi {
  return {
    available: false,
    start: async () => ({
      stop: async () => {},
      setTorch: async () => false,
    }),
  };
}

/** Checagem simples (§3.5): só URIs `localcamera://connect...` são aceitas.
 *  A validação completa é o parser Kotlin (F2-T05a). */
export function isPairingUri(raw: string): boolean {
  try {
    const u = new URL(raw);
    return u.protocol === "localcamera:" && u.host === "connect";
  } catch {
    return false;
  }
}
