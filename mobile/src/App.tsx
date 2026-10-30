// F2-T03 — Roteador por estado (ui/mobile.md §3.1) + botão voltar (Android)
// + sheet de Configurações + diálogo "Parar a transmissão?".
//
// A tela exibida é derivada de `selectScreen(state)`: não existe navegação
// imperativa — o estado da conexão (reducer local, F2-T13 no futuro) decide.

import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { App as CapApp } from "@capacitor/app";
import type { PluginListenerHandle } from "@capacitor/core";
import { Monitor, TriangleAlert, X } from "lucide-react";
import { BigToggle } from "@/components/BigToggle";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { useApp } from "@/state/AppProvider";
import { backAction, selectScreen } from "@/state/connection";
import { AwaitingApprovalScreen } from "@/screens/AwaitingApprovalScreen";
import { ConnectingScreen } from "@/screens/ConnectingScreen";
import { DefinitiveErrorScreen } from "@/screens/DefinitiveErrorScreen";
import { HomeScreen } from "@/screens/HomeScreen";
import { KnownPcsScreen } from "@/screens/KnownPcsScreen";
import { ManualScreen } from "@/screens/ManualScreen";
import { PermissionsScreen } from "@/screens/PermissionsScreen";
import { ScanScreen } from "@/screens/ScanScreen";
import { StreamingScreen } from "@/screens/StreamingScreen";

/** Configurações (bottom sheet de 85%, §3.11) — mínimo da Fase 2: só os
 *  interruptores de transmissão que o botão voltar já consome. */
function SettingsSheet({ onClose }: { onClose: () => void }) {
  const { t } = useTranslation();
  const { state, dispatch } = useApp();
  const { background, keepScreenOn } = state.settings;

  return (
    <div
      className="fixed inset-0 z-40 flex items-end bg-black/60"
      role="presentation"
      onClick={onClose}
    >
      <section
        role="dialog"
        aria-label={t("mobile.settings.title")}
        className="h-[85%] w-full overflow-y-auto rounded-t-xl border-t border-glass-border bg-bg-raised p-5"
        onClick={(e) => e.stopPropagation()}
      >
        <header className="mb-5 flex items-center justify-between">
          <h2 className="text-lg font-semibold">{t("mobile.settings.title")}</h2>
          <Button
            variant="ghost"
            size="icon"
            aria-label={t("common.actions.close")}
            onClick={onClose}
          >
            <X aria-hidden="true" />
          </Button>
        </header>

        <h3 className="mb-2 text-sm font-semibold text-text-muted">
          {t("mobile.settings.transmission")}
        </h3>
        <div className="flex flex-col gap-4">
          <div className="flex flex-col gap-1">
            <BigToggle
              icon={<Monitor className="h-5 w-5" aria-hidden="true" />}
              label={t("mobile.settings.background")}
              pressed={background}
              onClick={() => dispatch({ type: "SETTINGS", patch: { background: !background } })}
            />
            <p className="text-xs text-text-muted">{t("mobile.settings.backgroundHint")}</p>
          </div>
          <div className="flex flex-col gap-1">
            <BigToggle
              icon={<Monitor className="h-5 w-5" aria-hidden="true" />}
              label={t("mobile.settings.keepScreenOn")}
              pressed={keepScreenOn}
              onClick={() => dispatch({ type: "SETTINGS", patch: { keepScreenOn: !keepScreenOn } })}
            />
            <p className="text-xs text-text-muted">{t("mobile.settings.keepScreenOnHint")}</p>
          </div>
        </div>

        {!background && !keepScreenOn && (
          <p className="mt-4 flex items-center gap-2 rounded-md border border-warn/40 bg-warn/10 px-3 py-2 text-sm text-warn">
            <TriangleAlert className="h-4 w-4 shrink-0" aria-hidden="true" />
            {t("mobile.settings.bothOff")}
          </p>
        )}
      </section>
    </div>
  );
}

function App() {
  const { t } = useTranslation();
  const { state, dispatch } = useApp();
  const [stopConfirm, setStopConfirm] = useState(false);

  // O listener de `backButton` é registrado uma vez; lê o estado via ref.
  const stateRef = useRef(state);
  useEffect(() => {
    stateRef.current = state;
  }, [state]);

  useEffect(() => {
    let handle: PluginListenerHandle | undefined;
    let disposed = false;
    void CapApp.addListener("backButton", () => {
      switch (backAction(stateRef.current).kind) {
        case "closeSheet":
          dispatch({ type: "SHEET_CLOSE" });
          break;
        case "exitApp":
          // Lança "unimplemented" no web — sem efeito (navegador não tem app).
          void CapApp.exitApp().catch(() => {});
          break;
        case "goHome":
        case "cancelAndHome":
          // Cancelar a tentativa e sair do app equivalem a voltar à Home
          // (o reducer `GO_HOME` zera a conexão).
          dispatch({ type: "GO_HOME" });
          break;
        case "minimizeApp":
          void CapApp.minimizeApp().catch(() => {});
          break;
        case "confirmStop":
          setStopConfirm(true);
          break;
      }
    })
      .then((h) => {
        if (disposed) void h.remove().catch(() => {});
        else handle = h;
      })
      .catch(() => {});
    return () => {
      disposed = true;
      if (handle) void handle.remove().catch(() => {});
    };
  }, [dispatch]);

  // O diálogo só faz sentido em transmissão; se o estado sair de
  // streaming/reconnecting por outro meio, ele simplesmente não abre.
  const streaming =
    state.connection.kind === "streaming" || state.connection.kind === "reconnecting";
  const stopConfirmOpen = stopConfirm && streaming;

  const screen = selectScreen(state);

  return (
    <>
      {screen === "permissions" && <PermissionsScreen />}
      {screen === "home" && <HomeScreen />}
      {screen === "scan" && <ScanScreen />}
      {screen === "manual" && <ManualScreen />}
      {screen === "knownPcs" && <KnownPcsScreen />}
      {screen === "connecting" && <ConnectingScreen />}
      {screen === "awaiting" && <AwaitingApprovalScreen />}
      {screen === "streaming" && <StreamingScreen />}
      {screen === "error" && <DefinitiveErrorScreen />}

      {state.sheet === "settings" && (
        <SettingsSheet onClose={() => dispatch({ type: "SHEET_CLOSE" })} />
      )}

      <Dialog open={stopConfirmOpen} onOpenChange={setStopConfirm}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>{t("mobile.stream.stopConfirmTitle")}</DialogTitle>
          </DialogHeader>
          <DialogFooter>
            <Button variant="outline" onClick={() => setStopConfirm(false)}>
              {t("common.actions.cancel")}
            </Button>
            <Button
              variant="destructive"
              onClick={() => {
                setStopConfirm(false);
                dispatch({ type: "GO_HOME" });
              }}
            >
              {t("mobile.stream.stop")}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </>
  );
}

export default App;
