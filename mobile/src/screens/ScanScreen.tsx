// F2-T03 — Tela: Scan (QR) — ui/mobile.md §3.5.
//
// O leitor de câmera fica atrás de `qr/qr-scanner.ts` (stub até o scanner
// nativo de F2-T07/F2-T13). Só URIs `localcamera://connect...` são aceitas;
// QR inválido mostra a faixa `danger` por 3 s e não sai da tela.

import { useCallback, useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { ArrowLeft, Flashlight } from "lucide-react";
import { Button } from "@/components/ui/button";
import { useApp } from "@/state/AppProvider";
import { targetFromRawUri } from "@/state/connection";
import { isPairingUri, type QrScannerHandle } from "@/qr/qr-scanner";

const INVALID_MS = 3000;

export function ScanScreen() {
  const { t } = useTranslation();
  const { state, dispatch, qrScanner } = useApp();

  const [invalid, setInvalid] = useState(false);
  const [cameraError, setCameraError] = useState(false);
  const [torch, setTorch] = useState(false);
  const scannerRef = useRef<QrScannerHandle | null>(null);
  const videoRef = useRef<HTMLVideoElement | null>(null);
  const invalidTimerRef = useRef<number | null>(null);
  const lastInvalidRef = useRef<string | null>(null);
  const handledRef = useRef(false);

  const cameraDenied = state.permissions.camera === "denied";
  const canScan = qrScanner.available && !cameraDenied;

  const onCode = useCallback(
    (raw: string) => {
      if (handledRef.current) return;
      if (isPairingUri(raw)) {
        handledRef.current = true;
        lastInvalidRef.current = null;
        if (invalidTimerRef.current !== null) window.clearTimeout(invalidTimerRef.current);
        setInvalid(false);
        if (typeof navigator.vibrate === "function") void navigator.vibrate(50);
        dispatch({ type: "CONNECT", target: targetFromRawUri(raw), at: Date.now() });
        return;
      }
      if (lastInvalidRef.current === raw) return;
      lastInvalidRef.current = raw;
      setInvalid(true);
      if (invalidTimerRef.current !== null) window.clearTimeout(invalidTimerRef.current);
      invalidTimerRef.current = window.setTimeout(() => {
        lastInvalidRef.current = null;
        invalidTimerRef.current = null;
        setInvalid(false);
      }, INVALID_MS);
    },
    [dispatch],
  );

  useEffect(() => {
    if (!canScan) return;
    const video = videoRef.current;
    if (!video) return;

    let cancelled = false;
    setCameraError(false);
    void qrScanner
      .start(video, (raw) => {
        if (!cancelled) onCode(raw);
      })
      .then((handle) => {
        if (cancelled) {
          void handle.stop();
          return;
        }
        scannerRef.current = handle;
      })
      .catch(() => {
        if (!cancelled) setCameraError(true);
      });
    return () => {
      cancelled = true;
      const handle = scannerRef.current;
      scannerRef.current = null;
      void handle?.stop();
      if (invalidTimerRef.current !== null) window.clearTimeout(invalidTimerRef.current);
    };
  }, [canScan, qrScanner, onCode]);

  const toggleTorch = async () => {
    const next = !torch;
    try {
      if (await scannerRef.current?.setTorch(next)) setTorch(next);
    } catch {
      // Lanterna não suportada ou indisponível: mantém o estado visual atual.
    }
  };

  const cameraNotice = cameraDenied
    ? t("mobile.permissions.denied")
    : !qrScanner.available
      ? t("mobile.scan.unavailable")
      : cameraError
        ? t("mobile.scan.cameraError")
        : null;

  return (
    <main className="flex h-full flex-col gap-4 px-5 pb-6 pt-8">
      <header className="flex items-center justify-between">
        <Button
          variant="ghost"
          size="icon"
          aria-label={t("common.actions.back")}
          onClick={() => dispatch({ type: "ROUTE", route: "home" })}
        >
          <ArrowLeft aria-hidden="true" />
        </Button>
        <h1 className="text-lg font-semibold">{t("mobile.scan.title")}</h1>
        <Button
          variant={torch ? "default" : "ghost"}
          size="icon"
          aria-label={t("mobile.scan.torch")}
          aria-pressed={torch}
          onClick={() => void toggleTorch()}
        >
          <Flashlight aria-hidden="true" />
        </Button>
      </header>

      {invalid && (
        <div
          role="alert"
          className="rounded-md border border-danger/50 bg-danger/10 px-3 py-2 text-sm text-danger"
        >
          {t("mobile.scan.invalid")}
        </div>
      )}

      <div className="relative flex-1 overflow-hidden rounded-lg border border-glass-border bg-bg-raised">
        {canScan && (
          <video
            ref={videoRef}
            autoPlay
            muted
            playsInline
            aria-label={t("mobile.scan.cameraPreview")}
            className="absolute inset-0 h-full w-full object-cover"
          />
        )}
        <div aria-hidden="true" className="pointer-events-none absolute inset-0 m-auto h-44 w-44">
          <span className="absolute left-0 top-0 h-6 w-6 border-l-2 border-t-2 border-brand" />
          <span className="absolute right-0 top-0 h-6 w-6 border-r-2 border-t-2 border-brand" />
          <span className="absolute bottom-0 left-0 h-6 w-6 border-b-2 border-l-2 border-brand" />
          <span className="absolute bottom-0 right-0 h-6 w-6 border-b-2 border-r-2 border-brand" />
        </div>
        {cameraNotice && (
          <p
            role={cameraError || cameraDenied ? "alert" : "status"}
            className="absolute inset-x-5 top-5 rounded-md bg-background/90 px-3 py-2 text-center text-sm text-text-muted"
          >
            {cameraNotice}
          </p>
        )}
      </div>

      <p className="text-center text-sm text-text-muted">{t("mobile.scan.hint")}</p>

      <Button
        variant="outline"
        className="h-12 w-full"
        onClick={() => dispatch({ type: "ROUTE", route: "manual" })}
      >
        {t("mobile.manual.title")}
      </Button>
    </main>
  );
}
