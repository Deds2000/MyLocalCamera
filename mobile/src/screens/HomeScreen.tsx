// F2-T03 — Tela: Home (desconectado) — ui/mobile.md §3.4.

import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { ArrowRight, Cable, Monitor, QrCode, Settings, WifiOff } from "lucide-react";
import { Button } from "@/components/ui/button";
import { StatusChip } from "@/components/StatusChip";
import { useApp } from "@/state/AppProvider";
import { manualTarget, type KnownPc } from "@/state/connection";
import { formatSeen, useNowMs } from "./format";

/** `navigator.onLine` com atualização por eventos (proxy de rede; o status
 *  real de Wi-Fi/mobile chega com o plugin em F2-T13). */
function useOnline(): boolean {
  const [online, setOnline] = useState(() =>
    typeof navigator === "undefined" ? true : navigator.onLine,
  );
  useEffect(() => {
    if (typeof window === "undefined") return;
    const on = () => setOnline(true);
    const off = () => setOnline(false);
    window.addEventListener("online", on);
    window.addEventListener("offline", off);
    return () => {
      window.removeEventListener("online", on);
      window.removeEventListener("offline", off);
    };
  }, []);
  return online;
}

function ReconnectCard({ pc }: { pc: KnownPc }) {
  const { t, i18n } = useTranslation();
  const { dispatch } = useApp();
  const now = useNowMs();
  const seen = formatSeen(pc.lastSeenMs, now, i18n.resolvedLanguage);

  return (
    <div className="flex items-center justify-between gap-3 rounded-lg border border-glass-border bg-glass p-4">
      <div className="flex min-w-0 items-center gap-3">
        <Monitor className="h-5 w-5 shrink-0 text-brand" aria-hidden="true" />
        <div className="min-w-0">
          <p className="truncate text-sm font-medium">
            {t("mobile.home.reconnect", { name: pc.name })}
          </p>
          {seen && (
            <p className="text-xs text-text-muted">{t("mobile.home.seen", { when: seen })}</p>
          )}
        </div>
      </div>
      <Button
        size="sm"
        aria-label={t("mobile.home.reconnect", { name: pc.name })}
        // F2-T09 implementa a cadeia mDNS → últimos IPs → QR; na Fase 2 vai
        // direto para Conectando com o último endereço conhecido.
        onClick={() =>
          dispatch({
            type: "CONNECT",
            target: manualTarget(pc.host, pc.port, ""),
            at: Date.now(),
          })
        }
      >
        <ArrowRight aria-hidden="true" />
      </Button>
    </div>
  );
}

export function HomeScreen() {
  const { t } = useTranslation();
  const { state, dispatch } = useApp();
  const online = useOnline();

  const lastPc = state.knownPcs[state.knownPcs.length - 1];
  const missingPermission =
    state.permissions.mic !== "granted" || state.permissions.notifications !== "granted";
  const pcsLabel =
    state.knownPcs.length > 0
      ? `${t("mobile.home.knownPcs")} (${state.knownPcs.length})`
      : t("mobile.home.knownPcs");

  return (
    <main className="flex h-full flex-col gap-4 overflow-y-auto px-5 pb-6 pt-8">
      <header className="flex items-center justify-between">
        <h1 className="text-xl font-semibold">{t("common.appName")}</h1>
        <Button
          variant="ghost"
          size="icon"
          aria-label={t("mobile.settings.title")}
          onClick={() => dispatch({ type: "SHEET_OPEN", sheet: "settings" })}
        >
          <Settings aria-hidden="true" />
        </Button>
      </header>

      <div className="flex flex-col items-center gap-2 py-4 text-center">
        <StatusChip state="idle" label={t("common.state.idle")} />
        <h2 className="text-base text-text-muted">{t("mobile.home.title")}</h2>
      </div>

      <div className="flex flex-col gap-3">
        {!online && (
          <div className="flex items-center gap-3 rounded-lg border border-warn/40 bg-warn/10 p-4 text-warn">
            <WifiOff className="h-5 w-5 shrink-0" aria-hidden="true" />
            <p className="text-sm">{t("common.errors.noNetwork")}</p>
          </div>
        )}
        {missingPermission && (
          <div className="rounded-lg border border-warn/40 bg-warn/10 p-4 text-warn">
            <p className="text-sm">{t("mobile.permissions.denied")}</p>
          </div>
        )}
        {lastPc && <ReconnectCard key={lastPc.name} pc={lastPc} />}
        <div className="flex items-start gap-3 rounded-lg border border-glass-border bg-glass p-4">
          <Cable className="mt-0.5 h-5 w-5 shrink-0 text-text-muted" aria-hidden="true" />
          <p className="text-sm text-text-muted">{t("mobile.home.usbTip")}</p>
        </div>
      </div>

      <div className="mt-auto flex flex-col gap-2 pt-4">
        <Button className="h-14 w-full" onClick={() => dispatch({ type: "ROUTE", route: "scan" })}>
          <QrCode aria-hidden="true" />
          {t("mobile.home.scan")}
        </Button>
        <Button
          variant="outline"
          className="h-12 w-full"
          onClick={() => dispatch({ type: "ROUTE", route: "manual" })}
        >
          {t("mobile.home.manual")}
        </Button>
        <button
          type="button"
          className="mx-auto flex min-h-11 items-center gap-1 rounded-md text-sm text-text-muted focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
          onClick={() => dispatch({ type: "ROUTE", route: "knownPcs" })}
        >
          {pcsLabel}
          <ArrowRight className="h-4 w-4" aria-hidden="true" />
        </button>
      </div>
    </main>
  );
}
