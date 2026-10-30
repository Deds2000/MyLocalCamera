// F2-T03 — Tela: Permissões (primeira execução) — ui/mobile.md §3.3.
// Câmera é obrigatória; microfone e notificações são recomendados.

import { useTranslation } from "react-i18next";
import { Bell, Camera, Check, Mic, type LucideIcon } from "lucide-react";
import { Button } from "@/components/ui/button";
import { useApp } from "@/state/AppProvider";
import type { PermissionKind } from "@/state/connection";
import { cn } from "@/lib/utils";

interface PermissionRow {
  kind: PermissionKind;
  icon: LucideIcon;
  required: boolean;
}

const ROWS: PermissionRow[] = [
  { kind: "camera", icon: Camera, required: true },
  { kind: "mic", icon: Mic, required: false },
  { kind: "notifications", icon: Bell, required: false },
];

// Intent `openSettings` (Abrir configurações do Android) — o chamado nativo
// chega com o plugin Kotlin (F2-T13).
function openAndroidSettings(): void {
  // Ponto de integração F2-T13.
}

export function PermissionsScreen() {
  const { t } = useTranslation();
  const { state, dispatch } = useApp();

  const cameraGranted = state.permissions.camera === "granted";

  const grant = (kind: PermissionKind) =>
    // Fase 2: marca otimizamente; F2-T13 substitui pelo request nativo real.
    dispatch({ type: "PERMISSION", kind, status: "granted" });

  return (
    <main className="flex h-full flex-col gap-6 overflow-y-auto p-5">
      <header className="flex flex-col items-center gap-3 pt-8 text-center">
        <span
          aria-hidden="true"
          className="flex h-16 w-16 items-center justify-center rounded-2xl bg-glass border border-glass-border"
        >
          <Camera className="h-8 w-8 text-brand" />
        </span>
        <h1 className="text-2xl font-semibold">{t("mobile.permissions.title")}</h1>
      </header>

      <div className="flex flex-col gap-3">
        {ROWS.map(({ kind, icon: Icon, required }) => {
          const status = state.permissions[kind];
          const granted = status === "granted";
          const denied = status === "denied";
          return (
            <div
              key={kind}
              className="flex flex-col gap-2 rounded-lg border border-glass-border bg-glass p-4"
            >
              <div className="flex items-center justify-between gap-3">
                <div className="flex items-center gap-3">
                  <Icon className="h-5 w-5 shrink-0 text-brand" aria-hidden="true" />
                  <p className="text-sm text-text">{t(`mobile.permissions.${kind}`)}</p>
                </div>
                {granted ? (
                  <span className="flex items-center gap-1 text-sm text-ok">
                    <Check className="h-4 w-4" aria-hidden="true" />
                    {t("mobile.permissions.granted")}
                  </span>
                ) : denied ? (
                  <Button size="sm" variant="outline" onClick={() => openAndroidSettings()}>
                    {t("common.actions.openSettings")}
                  </Button>
                ) : (
                  <Button
                    size="sm"
                    variant={required ? "default" : "secondary"}
                    onClick={() => grant(kind)}
                  >
                    {t("mobile.permissions.grant")}
                  </Button>
                )}
              </div>
              {denied && <p className="text-xs text-warn">{t("mobile.permissions.denied")}</p>}
            </div>
          );
        })}
      </div>

      <div className="mt-auto flex flex-col gap-2 pb-2">
        {!cameraGranted && (
          <p className={cn("text-center text-xs", "text-warn")}>
            {t("mobile.permissions.cameraRequired")}
          </p>
        )}
        <Button
          className="h-12 w-full"
          disabled={!cameraGranted}
          onClick={() => dispatch({ type: "PERMISSIONS_DONE" })}
        >
          {t("mobile.permissions.continue")}
        </Button>
      </div>
    </main>
  );
}
