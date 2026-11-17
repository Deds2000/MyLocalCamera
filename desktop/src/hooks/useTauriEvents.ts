import { useEffect } from "react";
import type { UnlistenFn } from "@tauri-apps/api/event";
import type { TauriClient } from "@/lib/tauri";
import { appStore, type AppStore, type CameraStateEvent } from "@/store";
import type { StoreApi } from "zustand/vanilla";
import { tauri } from "@/lib/tauri";

/** The sole UI subscription point for Tauri events used by the UI. */
export function useTauriEvents(
  client: TauriClient = tauri,
  store: StoreApi<AppStore> = appStore,
): void {
  useEffect(() => {
    let disposed = false;
    const unlisteners: UnlistenFn[] = [];
    const subscribe = async () => {
      const subscriptions: Array<() => Promise<UnlistenFn>> = [
        () => client.onHealthChanged((event) => store.getState().applyHealthEvent(event)),
        () => client.onConnection((event) => {
          store.getState().applyConnectionEvent(event);
          store.getState().confirmTrustFromConnection(event);
        }),
        () => client.onStats((event) => store.getState().applyStatsEvent(event)),
        () => client.onPendingApproval((event) => store.getState().applyPendingApproval(event)),
        () => client.listen<CameraStateEvent>("session://state", (event) => store.getState().applyCameraState(event)),
      ];

      for (const [index, register] of subscriptions.entries()) {
        try {
          const unlisten = await register();
          if (disposed) unlisten();
          else {
            unlisteners.push(unlisten);
            if (index === 0) void store.getState().loadHealth();
          }
        } catch {
          if (!disposed) {
            store.getState().setConnectionError("EVENT_SUBSCRIPTION_FAILED");
            if (index === 0) void store.getState().loadHealth();
          }
        }
      }
    };

    void store.getState().loadTrustedDevices();
    void store.getState().loadConnectionInfo();
    void subscribe();

    return () => {
      disposed = true;
      unlisteners.forEach((unlisten) => unlisten());
    };
  }, [client, store]);
}
