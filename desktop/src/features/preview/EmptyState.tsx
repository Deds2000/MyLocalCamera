import { useTranslation } from "react-i18next";
import { PairingQrCard } from "@/features/connection/PairingQrCard";

export function EmptyState() {
  const { t } = useTranslation();

  return (
    <div
      data-testid="preview-empty-state"
      className="mx-auto flex min-h-full w-full flex-col items-center justify-center gap-1 text-center"
    >
      <div className="space-y-1">
        <h2 className="text-base font-semibold tracking-tight">{t("desktop.empty.title")}</h2>
        <p className="text-xs text-text-muted">{t("desktop.empty.subtitle")}</p>
      </div>
      <PairingQrCard showCode={false} compact showInterfaceSelector={false} />
      <div className="flex w-full items-center gap-3 text-xs text-text-muted" aria-hidden="true">
        <span className="h-px flex-1 bg-glass-border" />
        <span>{t("desktop.empty.usbSeparator")}</span>
        <span className="h-px flex-1 bg-glass-border" />
      </div>
      <p className="text-xs leading-4 text-text-muted">{t("desktop.empty.usbHint")}</p>
    </div>
  );
}
