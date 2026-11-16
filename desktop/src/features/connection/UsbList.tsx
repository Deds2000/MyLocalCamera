import { Usb } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Switch } from "@/components/ui/switch";
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from "@/components/ui/tooltip";

export function UsbList() {
  const { t } = useTranslation();

  return (
    <section aria-labelledby="usb-list-title" className="space-y-3">
      <h3 id="usb-list-title" className="text-sm font-semibold">{t("desktop.connection.usbTitle")}</h3>
      <div className="rounded-lg border border-glass-border bg-glass p-3">
        <div className="flex items-start gap-2 text-sm text-text-muted">
          <Usb className="mt-0.5 size-4 shrink-0" aria-hidden="true" />
          <p>{t("desktop.connection.usbNone")}</p>
        </div>
        <p className="mt-2 pl-6 text-xs text-text-muted">{t("desktop.empty.usbHint")}</p>
        <div className="mt-3 flex items-center justify-between gap-3 border-t border-glass-border pt-3">
          <span id="usb-auto-label" className="text-xs text-text-muted">
            {t("desktop.connection.usbAuto")}
          </span>
          <TooltipProvider>
            <Tooltip>
              <TooltipTrigger asChild>
                <span tabIndex={0} aria-describedby="usb-auto-help">
                  <Switch aria-labelledby="usb-auto-label" disabled checked={false} />
                </span>
              </TooltipTrigger>
              <TooltipContent id="usb-auto-help">
                {t("desktop.connection.usbAutoHelp")}
              </TooltipContent>
            </Tooltip>
          </TooltipProvider>
        </div>
      </div>
    </section>
  );
}
