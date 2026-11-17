import { useEffect, useRef } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from "@/components/ui/tooltip";
import type { HealthSnapshot } from "@/store/health";

type ServerState = "listening" | "altPort" | "failed";

interface HealthPopoverProps {
  state: ServerState;
  port: number;
  health: HealthSnapshot;
  onRetry: () => void;
  onClose: () => void;
  triggerRef: React.RefObject<HTMLButtonElement | null>;
}

export function HealthPopover({
  state,
  port,
  health,
  onRetry,
  onClose,
  triggerRef,
}: HealthPopoverProps) {
  const { t } = useTranslation();
  const rootRef = useRef<HTMLDivElement>(null);
  const id = "health-server-popover";

  useEffect(() => {
    const onClick = (event: MouseEvent) => {
      if (
        !rootRef.current?.contains(event.target as Node) &&
        !triggerRef.current?.contains(event.target as Node)
      )
        onClose();
    };
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        event.preventDefault();
        onClose();
        triggerRef.current?.focus();
      }
    };
    document.addEventListener("click", onClick);
    document.addEventListener("keydown", onKeyDown);
    return () => {
      document.removeEventListener("click", onClick);
      document.removeEventListener("keydown", onKeyDown);
    };
  }, [onClose, triggerRef]);

  const detailKey = `desktop.health.detail.${state === "listening" ? "ok" : state === "failed" ? "down" : "altPort"}`;
  return (
    <div
      ref={rootRef}
      id={id}
      role="dialog"
      aria-modal="false"
      aria-label={t("desktop.health.aria.serverDetails")}
      data-testid="health-popover"
      className="absolute bottom-full left-0 z-40 mb-2 w-64 rounded-md border border-glass-border bg-popover p-3 text-popover-foreground shadow-md"
    >
      <p className="mb-3 text-sm">{t(detailKey, { port })}</p>
      {state === "altPort" && (
        <TooltipProvider>
          <Tooltip>
            <TooltipTrigger asChild>
              <span className="inline-flex" tabIndex={0} aria-describedby="health-settings-tooltip">
                <Button type="button" disabled>
                  {t("desktop.health.openSettings")}
                </Button>
              </span>
            </TooltipTrigger>
            <TooltipContent id="health-settings-tooltip">
              {t("desktop.health.settingsSoon")}
            </TooltipContent>
          </Tooltip>
        </TooltipProvider>
      )}
      {state === "failed" && (
        <div>
          <Button type="button" onClick={onRetry} disabled={health.restarting}>
            {health.restarting ? t("desktop.health.retrying") : t("desktop.health.retry")}
          </Button>
          {health.errorCode === "HEALTH_RESTART_FAILED" && (
            <p role="status" className="mt-2 text-sm text-danger">
              {t("desktop.health.retryFailed")}
            </p>
          )}
        </div>
      )}
    </div>
  );
}
