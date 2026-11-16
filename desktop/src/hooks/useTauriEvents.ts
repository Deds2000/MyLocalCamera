import { useEffect } from "react";
import type { UnlistenFn } from "@tauri-apps/api/event";
import type { TauriClient } from "@/lib/tauri";
import { appStore, type AppStore, type CameraStateEvent } from "@/store";
import type { StoreApi } from "zustand/vanilla";
import { tauri } from "@/lib/tauri";

/** The sole UI subscription point for session/trust events. */
export function useTauriEvents(
  client: TauriClient = tauri,
  store: StoreApi<AppStore> = appStore,
): void {
  useEffect(() => {
    let disposed = false;
    const unlisteners: UnlistenFn[] = [];
    const subscribe = async () => {
      const subscriptions: Array<() => Promise<UnlistenFn>> = [
        () => client.onConnection((event) => {
          store.getState().applyConnectionEvent(event);
          store.getState().confirmTrustFromConnection(event);
        }),
        () => client.onStats((event) => store.getState().applyStatsEvent(event)),
        () => client.onPendingApproval((event) => store.getState().applyPendingApproval(event)),
        () => client.listen<CameraStateEvent>("session://state", (event) => store.getState().applyCameraState(event)),
      ];

      for (const register of subscriptions) {
        try {
          const unlisten = await register();
          if (disposed) unlisten();
          else unlisteners.push(unlisten);
        } catch {
          store.getState().setConnectionError("EVENT_SUBSCRIPTION_FAILED");
        }
      }
    };

    void store.getState().loadTrustedDevices();
    void store.getState().loadHealth();
    void store.getState().loadConnectionInfo();
    void subscribe();

    return () => {
      disposed = true;
      unlisteners.forEach((unlisten) => unlisten());
    };
  }, [client, store]);
}
