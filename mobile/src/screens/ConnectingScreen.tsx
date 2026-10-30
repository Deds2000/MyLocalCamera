// F2-T03 — Tela: Conectando — ui/mobile.md §3.7.
// Timeout de 10 s → estado de falha com "Tentar de novo" / "Escanear QR".
// (Em produção o timeout/erro vêm do plugin, F2-T13.)

import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Loader2, TriangleAlert } from "lucide-react";
import { Button } from "@/components/ui/button";
import { useApp } from "@/state/AppProvider";

const TIMEOUT_MS = 10_000;

export function ConnectingScreen() {
  const { state } = useApp();
  const conn = state.connection;
  const at = conn.kind === "connecting" ? conn.at : 0;
  const name = conn.kind === "connecting" ? conn.target.name : "";
  // Cada nova tentativa (novo `at`) remonta a view: o estado de falha zera
  // e o timeout recomeça — sem efeito de reset no componente.
  return <ConnectingView key={at} at={at} name={name} />;
}

function ConnectingView({ at, name }: { at: number; name: string }) {
  const { t } = useTranslation();
  const { state, dispatch } = useApp();
  const target = state.connection.kind === "connecting" ? state.connection.target : null;
  const [failed, setFailed] = useState(false);

  useEffect(() => {
    if (failed || at === 0) return;
    const id = window.setTimeout(() => setFailed(true), TIMEOUT_MS);
    return () => window.clearTimeout(id);
  }, [failed, at]);

  return (
    <main className="flex h-full flex-col items-center justify-center gap-6 px-5 text-center">
      {failed ? (
        <>
          <TriangleAlert className="h-12 w-12 text-danger" aria-hidden="true" />
          <p className="text-base text-text">{t("mobile.connecting.failed")}</p>
          <div className="flex w-full max-w-xs flex-col gap-2">
            <Button
              className="h-12 w-full"
              onClick={() => target && dispatch({ type: "CONNECT", target, at: Date.now() })}
            >
              {t("common.actions.retry")}
            </Button>
            <Button
              variant="outline"
              className="h-12 w-full"
              onClick={() => {
                dispatch({ type: "GO_HOME" });
                dispatch({ type: "ROUTE", route: "scan" });
              }}
            >
              {t("mobile.scan.title")}
            </Button>
          </div>
        </>
      ) : (
        <>
          <Loader2
            className="h-12 w-12 animate-spin text-brand motion-reduce:animate-none"
            aria-hidden="true"
          />
          <h1 className="text-lg font-medium">
            {name ? t("mobile.connecting.title", { name }) : t("common.state.connecting")}
          </h1>
          <div className="w-full max-w-xs">
            <Button
              variant="outline"
              className="h-12 w-full"
              onClick={() => dispatch({ type: "GO_HOME" })}
            >
              {t("common.actions.cancel")}
            </Button>
          </div>
        </>
      )}
    </main>
  );
}
