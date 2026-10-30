// F2-T03 — Tela: Erro definitivo — ui/mobile.md §3.10.
//
// Layout único (EmptyState em tela cheia): ícone na cor do erro, título,
// explicação e 1 ação primária + 1 secundária. Tudo por CÓDIGO — nenhum
// texto da rede chega à UI (UT-2.09).

import { useTranslation } from "react-i18next";
import { TriangleAlert, WifiOff } from "lucide-react";
import { Button } from "@/components/ui/button";
import { useApp } from "@/state/AppProvider";
import { errorBodyKey, reasonToI18nKey } from "@/state/connection";

type ErrorButtonKind = "scan" | "manual" | "retry" | "back" | "openSettings" | "usb";

/** Tabela §3.10: ação primária/secundária por código de erro. */
function errorActions(reason: string): {
  primary: ErrorButtonKind;
  secondary: ErrorButtonKind | null;
} {
  switch (reason) {
    case "bad_token":
      return { primary: "scan", secondary: "manual" };
    case "not_approved":
      return { primary: "retry", secondary: "back" };
    case "revoked":
      return { primary: "scan", secondary: "back" };
    case "busy":
      return { primary: "retry", secondary: "back" };
    case "version_mismatch":
      return { primary: "back", secondary: null };
    case "no_network":
      // Código interno (não vem da rede): sem rede → Wi-Fi ou USB.
      return { primary: "openSettings", secondary: "usb" };
    default:
      // timeout, protocol_error, bad_audio_ticket e qualquer código novo.
      return { primary: "retry", secondary: "back" };
  }
}

const BUTTON_LABEL: Record<ErrorButtonKind, string> = {
  scan: "mobile.scan.title",
  manual: "mobile.manual.title",
  retry: "common.actions.retry",
  back: "common.actions.back",
  openSettings: "common.actions.openSettings",
  usb: "common.transport.usb",
};

export function DefinitiveErrorScreen() {
  const { t } = useTranslation();
  const { state, dispatch } = useApp();

  const conn = state.connection;
  if (conn.kind !== "error") return null;

  const { reason, target, versions, versionSide } = conn;
  const isNoNetwork = reason === "no_network";
  const { primary, secondary } = errorActions(reason);

  const title = isNoNetwork
    ? t("common.errors.noNetwork")
    : t(`mobile.error.${reasonToI18nKey(reason)}.title`);
  const body = isNoNetwork
    ? t("mobile.home.usbTip")
    : reason === "version_mismatch"
      ? t(errorBodyKey(reason, versionSide), {
          phone: versions?.phone ?? "?",
          pc: versions?.pc ?? "?",
        })
      : t(errorBodyKey(reason));

  // Intent `openSettings` (Wi-Fi) — o chamado nativo chega com F2-T13.
  const act = (kind: ErrorButtonKind) => {
    switch (kind) {
      case "scan":
        dispatch({ type: "GO_HOME" });
        dispatch({ type: "ROUTE", route: "scan" });
        return;
      case "manual":
        dispatch({ type: "GO_HOME" });
        dispatch({ type: "ROUTE", route: "manual" });
        return;
      case "usb":
        // Leva à Home, onde o cartão de cabo USB dá a instrução.
        dispatch({ type: "GO_HOME" });
        return;
      case "back":
        dispatch({ type: "GO_HOME" });
        return;
      case "retry":
      case "openSettings":
        return;
    }
  };

  const label = (kind: ErrorButtonKind) => t(BUTTON_LABEL[kind]);

  return (
    <main className="flex h-full flex-col items-center justify-center gap-5 px-6 text-center">
      {isNoNetwork ? (
        <WifiOff className="h-14 w-14 text-danger" aria-hidden="true" />
      ) : (
        <TriangleAlert className="h-14 w-14 text-danger" aria-hidden="true" />
      )}

      <div className="flex max-w-sm flex-col gap-2">
        <h1 className="text-xl font-semibold">{title}</h1>
        <p className="text-sm text-text-muted">{body}</p>
      </div>

      <div className="flex w-full max-w-xs flex-col gap-2">
        {primary === "retry" ? (
          // Reconecta para o mesmo alvo (em produção, via plugin — F2-T13).
          <Button
            className="h-12 w-full"
            onClick={() =>
              target
                ? dispatch({ type: "CONNECT", target, at: Date.now() })
                : dispatch({ type: "GO_HOME" })
            }
          >
            {label(primary)}
          </Button>
        ) : (
          <Button className="h-12 w-full" onClick={() => act(primary)}>
            {label(primary)}
          </Button>
        )}
        {secondary && (
          <Button variant="outline" className="h-12 w-full" onClick={() => act(secondary)}>
            {label(secondary)}
          </Button>
        )}
      </div>
    </main>
  );
}
