// F2-T03 — Tela: PCs conhecidos — ui/mobile.md §3.9.
// Lista de DeviceRow (nome, "visto há…", Conectar, Esquecer c/ confirmação).

import { useState } from "react";
import { useTranslation } from "react-i18next";
import { ArrowLeft, Monitor } from "lucide-react";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { useApp } from "@/state/AppProvider";
import { manualTarget, type KnownPc } from "@/state/connection";
import { formatSeen, useNowMs } from "./format";

export function KnownPcsScreen() {
  const { t, i18n } = useTranslation();
  const { state, dispatch } = useApp();
  const [forgetTarget, setForgetTarget] = useState<KnownPc | null>(null);
  const now = useNowMs();

  const confirmForget = () => {
    if (forgetTarget) dispatch({ type: "KPC_FORGET", name: forgetTarget.name });
    setForgetTarget(null);
  };

  return (
    <main className="flex h-full flex-col gap-4 overflow-y-auto px-5 pb-6 pt-8">
      <header className="flex items-center gap-3">
        <Button
          variant="ghost"
          size="icon"
          aria-label={t("common.actions.back")}
          onClick={() => dispatch({ type: "ROUTE", route: "home" })}
        >
          <ArrowLeft aria-hidden="true" />
        </Button>
        <h1 className="text-lg font-semibold">{t("mobile.knownPcs.title")}</h1>
      </header>

      {state.knownPcs.length === 0 ? (
        <div className="flex flex-1 flex-col items-center justify-center gap-4 text-center">
          <Monitor className="h-10 w-10 text-text-muted" aria-hidden="true" />
          <p className="text-sm text-text-muted">{t("mobile.knownPcs.empty")}</p>
          <Button
            className="h-12 w-full max-w-xs"
            onClick={() => dispatch({ type: "ROUTE", route: "scan" })}
          >
            {t("mobile.scan.title")}
          </Button>
        </div>
      ) : (
        <ul className="flex flex-col gap-3">
          {state.knownPcs.map((pc) => {
            const seen = formatSeen(pc.lastSeenMs, now, i18n.resolvedLanguage);
            return (
              <li
                key={pc.name}
                className="flex flex-col gap-3 rounded-lg border border-glass-border bg-glass p-4"
              >
                <div className="flex items-center gap-3">
                  <Monitor className="h-5 w-5 shrink-0 text-brand" aria-hidden="true" />
                  <div className="min-w-0">
                    <p className="truncate text-sm font-medium">{pc.name}</p>
                    {seen && (
                      <p className="text-xs text-text-muted">
                        {t("mobile.home.seen", { when: seen })}
                      </p>
                    )}
                  </div>
                </div>
                <div className="flex gap-2">
                  <Button
                    size="sm"
                    className="flex-1"
                    onClick={() =>
                      dispatch({
                        type: "CONNECT",
                        // F2-T09: cadeia mDNS → últimos IPs → QR; na Fase 2
                        // vai direto para Conectando com o último endereço.
                        target: manualTarget(pc.host, pc.port, ""),
                        at: Date.now(),
                      })
                    }
                  >
                    {t("mobile.manual.connect")}
                  </Button>
                  <Button
                    size="sm"
                    variant="outline"
                    className="flex-1"
                    onClick={() => setForgetTarget(pc)}
                  >
                    {t("mobile.knownPcs.forget")}
                  </Button>
                </div>
              </li>
            );
          })}
        </ul>
      )}

      <Dialog open={forgetTarget !== null} onOpenChange={(open) => !open && setForgetTarget(null)}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>
              {t("mobile.knownPcs.forgetConfirm", { name: forgetTarget?.name ?? "" })}
            </DialogTitle>
          </DialogHeader>
          <DialogFooter>
            <Button variant="outline" onClick={() => setForgetTarget(null)}>
              {t("common.actions.cancel")}
            </Button>
            <Button variant="destructive" onClick={confirmForget}>
              {t("mobile.knownPcs.forget")}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </main>
  );
}
