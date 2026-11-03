// F2-T03 — Scanner QR contínuo no vídeo da própria WebView.
// Capacitor BridgeWebChromeClient encaminha VIDEO_CAPTURE à permissão Android;
// nenhum navegador externo é iniciado. O fallback continua sendo a tela Manual.

import type { IScannerControls } from "@zxing/browser";

export interface QrScannerHandle {
  stop(): Promise<void>;
  /** Alterna a lanterna; devolve `false` se não suportado. */
  setTorch(on: boolean): Promise<boolean>;
}

export interface QrScannerApi {
  /** `false` quando o runtime não oferece captura de vídeo. */
  readonly available: boolean;
  /** Inicia a leitura; `onCode` recebe o texto bruto do QR. */
  start(video: HTMLVideoElement, onCode: (raw: string) => void): Promise<QrScannerHandle>;
}

/** Usa a câmera traseira e a prévia de vídeo montada pela ScanScreen. */
export function getQrScanner(): QrScannerApi {
  const available =
    typeof navigator !== "undefined" && typeof navigator.mediaDevices?.getUserMedia === "function";

  return {
    available,
    start: async (video, onCode) => {
      if (!available) throw new Error("camera_unavailable");

      const { BrowserQRCodeReader } = await import("@zxing/browser");
      const reader = new BrowserQRCodeReader();
      let controls: IScannerControls | null = null;
      let stopped = false;

      controls = await reader.decodeFromVideoDevice(undefined, video, (result) => {
        if (!stopped && result) onCode(result.getText());
      });

      return {
        stop: async () => {
          stopped = true;
          controls?.stop();
        },
        setTorch: async (on) => {
          if (stopped || !controls?.switchTorch) return false;
          try {
            await controls.switchTorch(on);
            return true;
          } catch {
            return false;
          }
        },
      };
    },
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
