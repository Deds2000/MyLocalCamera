import { useEffect } from "react";
import { useTranslation } from "react-i18next";
import { useStore } from "zustand";
import type { StoreApi } from "zustand/vanilla";
import { appStore, selectConnection, selectStats, type AppStore } from "@/store";
import { DeviceCard } from "./DeviceCard";
import { PairingQrCard } from "./PairingQrCard";
import { UsbList } from "./UsbList";
import { TrustedList } from "./TrustedList";

interface ConnectionTabProps {
  store?: StoreApi<AppStore>;
}

export function ConnectionTab({ store = appStore }: ConnectionTabProps) {
  const { t } = useTranslation();
  const connection = useStore(store, selectConnection);
  const stats = useStore(store, selectStats);

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
            <h3 id="pair-title" className="text-sm font-semibold">
              {t("desktop.connection.pairTitle")}
            </h3>
            <PairingQrCard store={store} />
          </section>
        )}
        <UsbList />
        <TrustedList store={store} />
      </div>
    </div>
  );
}
