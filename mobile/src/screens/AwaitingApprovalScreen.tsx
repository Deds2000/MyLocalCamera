// F2-T03 — Tela: Aguardando aprovação — ui/mobile.md §3.7.
// CountdownRing (30 s por padrão); ao zerar sem resposta → erro `not_approved`.

import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { CountdownRing } from "@/components/CountdownRing";
import { useApp } from "@/state/AppProvider";

const TICK_MS = 1000;

export function AwaitingApprovalScreen() {
  const { t } = useTranslation();
  const { state, dispatch } = useApp();

  const conn = state.connection;
  const total = conn.kind === "pending" ? conn.timeoutS : 30;
  // Uma nova instância de `pending` sempre remonta a tela (o reducer só
  // entra em `pending` a partir de `connecting`), então o valor inicial
  // já está correto sem efeito de reset.
  const [remaining, setRemaining] = useState(total);

  useEffect(() => {
    const id = window.setInterval(() => {
      setRemaining((s) => (s > 0 ? s - 1 : 0));
    }, TICK_MS);
    return () => window.clearInterval(id);
  }, []);

  useEffect(() => {
    if (remaining === 0) dispatch({ type: "ERROR", reason: "not_approved" });
  }, [remaining, dispatch]);

  return (
    <main className="flex h-full flex-col items-center justify-center gap-5 px-5 text-center">
      <div className="relative">
        <CountdownRing
          seconds={remaining}
          total={total}
          size={112}
          strokeWidth={5}
          ariaLabel={t("mobile.pending.title")}
        />
        <span
          aria-hidden="true"
          className="absolute inset-0 flex items-center justify-center font-mono text-3xl tabular-nums"
        >
          {remaining}
        </span>
      </div>

      <div className="flex flex-col gap-2">
        <h1 className="text-xl font-semibold">{t("mobile.pending.title")}</h1>
        <p className="max-w-xs text-sm text-text-muted">{t("mobile.pending.body")}</p>
      </div>

      <div className="w-full max-w-xs">
        <Button
          variant="outline"
          className="h-12 w-full"
          onClick={() => dispatch({ type: "GO_HOME" })}
        >
          {t("common.actions.cancel")}
        </Button>
      </div>
    </main>
  );
}
