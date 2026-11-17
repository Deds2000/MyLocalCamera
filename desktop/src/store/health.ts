import type { HealthInfo, TauriClient } from "@/lib/tauri";
import type { AppSet } from "./types";

export interface HealthSnapshot {
  data: HealthInfo | null;
  loading: boolean;
  errorCode: string | null;
  restarting: boolean;
}

export interface HealthSlice {
  health: HealthSnapshot;
  loadHealth: () => Promise<void>;
  applyHealthEvent: (info: HealthInfo) => void;
  restartServer: () => Promise<void>;
}

export function createHealthSlice(set: AppSet, client: TauriClient): HealthSlice {
  let revision = 0;
  return {
    health: { data: null, loading: false, errorCode: null, restarting: false },
    loadHealth: async () => {
      const requestRevision = revision;
      set((state) => ({ health: { ...state.health, loading: true, errorCode: null } }));
      try {
        const data = await client.getHealth();
        set((state) => ({
          health: {
            ...state.health,
            ...(requestRevision === revision ? { data, errorCode: null } : {}),
            loading: false,
          },
        }));
      } catch {
        set((state) => ({
          health: {
            ...state.health,
            loading: false,
            ...(requestRevision === revision ? { errorCode: "HEALTH_LOAD_FAILED" } : {}),
          },
        }));
      }
    },
    applyHealthEvent: (info) => {
      revision += 1;
      set((state) => ({ health: { ...state.health, data: info, errorCode: null } }));
    },
    restartServer: async () => {
      set((state) => ({ health: { ...state.health, restarting: true, errorCode: null } }));
      try {
        await client.restartServer();
        set((state) => ({ health: { ...state.health, restarting: false } }));
      } catch {
        set((state) => ({
          health: { ...state.health, restarting: false, errorCode: "HEALTH_RESTART_FAILED" },
        }));
      }
    },
  };
}
