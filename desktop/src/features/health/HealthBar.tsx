import { useCallback, useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { useStore } from "zustand";
import type { StoreApi } from "zustand/vanilla";
import { HealthIndicator } from "@/components/HealthIndicator";
import { appStore, selectHealth, type AppStore } from "@/store";
import { HealthPopover } from "./HealthPopover";

const futureKinds = ["discovery", "vcam", "mic", "adb"] as const;
type ServerState = "listening" | "altPort" | "failed";

export function HealthBar({ store = appStore }: { store?: StoreApi<AppStore> }) {
  const { t } = useTranslation();
  const health = useStore(store, selectHealth);
  const [open, setOpen] = useState(false);
  const triggerRef = useRef<HTMLButtonElement>(null);
  const server = health.data?.server;
  const state: ServerState =
    server?.state === "listening"
      ? "listening"
      : server?.state === "altPort"
        ? "altPort"
        : "failed";
  const port = server?.effectivePort ?? server?.port ?? 0;
  const detail =
    state === "listening"
      ? t("desktop.health.server.ok", { port })
      : state === "altPort"
        ? t("desktop.health.server.altPort", { port })
        : t("desktop.health.server.down");
  const serverLabel =
    state === "altPort" ? `${t("desktop.health.label.server")} ${detail}` : detail;
  const close = useCallback(() => {
    setOpen(false);
    triggerRef.current?.focus();
  }, []);

  const retry = () => {
    const state = store.getState();
    if (state.health.data === null) void state.loadHealth();
    else void state.restartServer();
  };

  useEffect(() => {
    if (health.data === null && !health.loading && health.errorCode === null)
      void store.getState().loadHealth();
  }, [health.data, health.loading, health.errorCode, store]);

  return (
    <div
      role="group"
      data-testid="health-bar"
      aria-label={t("desktop.health.aria.bar")}
      className="relative flex h-8 min-w-0 items-center gap-1 px-2"
    >
      <div className="relative min-w-0 shrink">
        <HealthIndicator
          kind="server"
          state={state === "listening" ? "ok" : state === "altPort" ? "warn" : "danger"}
          detail={detail}
          onClick={() => setOpen((current) => !current)}
          aria-label={serverLabel}
          aria-expanded={open}
          aria-haspopup="dialog"
          aria-controls="health-server-popover"
          buttonRef={triggerRef}
          className="max-w-full"
        />
        {open && (
          <HealthPopover
            state={state}
            port={port}
            health={health}
            onRetry={retry}
            onClose={close}
            triggerRef={triggerRef}
          />
        )}
      </div>
      {futureKinds.map((kind) => (
        <HealthIndicator
          key={kind}
          kind={kind}
          state="muted"
          detail={t("desktop.health.soonItem", {
            item: t(`desktop.health.label.${kind}`),
            soon: t("desktop.health.soon"),
          })}
          className="min-w-0 flex-1 justify-center"
        />
      ))}
    </div>
  );
}
