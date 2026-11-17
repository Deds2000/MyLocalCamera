import { Camera, Wifi } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from "@/components/ui/tooltip";
import type { ConnectionSnapshot } from "@/store/connection";

interface DeviceCardProps {
  connection: ConnectionSnapshot;
  rttMs: number | null;
}

export function DeviceCard({ connection, rttMs }: DeviceCardProps) {
  const { t } = useTranslation();
  const device = connection.device;
  if (connection.state !== "connected" || !device) return null;

  return (
    <section
      data-testid="connected-device-card"
      aria-labelledby="connected-device-title"
      className="rounded-lg border border-glass-border bg-glass p-3"
    >
      <div className="flex items-start gap-2.5">
        <Camera className="mt-0.5 size-5 shrink-0 text-brand" aria-hidden="true" />
        <div className="min-w-0 flex-1">
          <h3 id="connected-device-title" className="truncate text-sm font-semibold">
            {device.name}
          </h3>
          {device.model && <p className="truncate text-xs text-text-muted">{device.model}</p>}
          <div className="mt-2 flex flex-wrap items-center gap-2 text-xs text-text-muted">
            {connection.transport && (
              <span className="inline-flex items-center gap-1 rounded-full border border-glass-border px-2 py-0.5">
                {connection.transport === "tcp" && <Wifi className="size-3.5" aria-hidden="true" />}
                {t(
                  connection.transport === "tcp" ? "common.transport.wifi" : "common.transport.usb",
                )}
              </span>
            )}
            {rttMs !== null && <span>{t("common.units.ms", { value: Math.round(rttMs) })}</span>}
            {device.ip && <span className="font-mono">{device.ip}</span>}
          </div>
        </div>
        <TooltipProvider>
          <Tooltip>
            <TooltipTrigger asChild>
              <span tabIndex={0} aria-describedby="disconnect-unavailable-help">
                <Button type="button" variant="secondary" size="sm" disabled>
                  {t("desktop.connection.disconnect")}
                </Button>
              </span>
            </TooltipTrigger>
            <TooltipContent id="disconnect-unavailable-help">
              {t("desktop.connection.disconnectUnavailable")}
            </TooltipContent>
          </Tooltip>
        </TooltipProvider>
      </div>
    </section>
  );
}
