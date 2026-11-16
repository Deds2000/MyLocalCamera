import { useEffect } from "react";
import { useTranslation } from "react-i18next";
import { useStore } from "zustand";
import type { StoreApi } from "zustand/vanilla";
import { appStore, selectConnection, selectStats, type AppStore } from "@/store";
import { DeviceCard } from "./DeviceCard";
import { PairingQrCard } from "./PairingQrCard";
import { UsbList } from "./UsbList";

interface ConnectionTabProps {
  store?: StoreApi<AppStore>;
}

export function ConnectionTab({ store = appStore }: ConnectionTabProps) {
  const { t } = useTranslation();
  const connection = useStore(store, selectConnection);
  const stats = useStore(store, selectStats);
  const trustedDevices = useStore(store, (state) => state.trust.trusted);

  useEffect(() => {
    if (!store.getState().connectionInfo && !store.getState().connectionInfoLoading) {
      void store.getState().loadConnectionInfo();
    }
  }, [store]);

  return (
    <div data-testid="connection-tab" className="h-full overflow-y-auto p-4">
      <div className="mx-auto flex max-w-sm flex-col gap-5">
        <h2 className="text-base font-semibold">{t("desktop.tabs.connection")}</h2>
        <DeviceCard connection={connection} rttMs={stats?.rttMs ?? null} />
        {connection.state !== "connected" && (
          <section aria-labelledby="pair-title" data-testid="pairing-section" className="space-y-3">
            <h3 id="pair-title" className="text-sm font-semibold">{t("desktop.connection.pairTitle")}</h3>
            <PairingQrCard store={store} />
          </section>
        )}
        <UsbList />
        <section aria-labelledby="trusted-title" className="space-y-3">
          <h3 id="trusted-title" className="text-sm font-semibold">{t("desktop.trusted.title")}</h3>
          {trustedDevices.length === 0 ? (
            <p className="rounded-lg border border-glass-border bg-glass p-3 text-sm text-text-muted">
              {t("desktop.trusted.empty")}
            </p>
          ) : (
            <ul className="space-y-2" aria-label={t("desktop.trusted.title")}>
              {trustedDevices.map((device) => (
                <li key={device.deviceId} className="rounded-lg border border-glass-border bg-glass p-3">
                  <p className="truncate text-sm font-medium">{device.name}</p>
                  <p className="truncate text-xs text-text-muted">
                    {[device.model, device.lastIp].filter(Boolean).join(" · ")}
                  </p>
                </li>
              ))}
            </ul>
          )}
        </section>
      </div>
    </div>
  );
}
