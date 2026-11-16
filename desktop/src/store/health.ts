import type { HealthInfo, TauriClient } from "@/lib/tauri";
import type { AppSet } from "./types";

export interface HealthSnapshot {
  data: HealthInfo | null;
  loading: boolean;
  errorCode: string | null;
}

export interface HealthSlice {
  health: HealthSnapshot;
  loadHealth: () => Promise<void>;
}

export function createHealthSlice(set: AppSet, client: TauriClient): HealthSlice {
  return {
    health: { data: null, loading: false, errorCode: null },
    loadHealth: async () => {
      set((state) => ({ health: { ...state.health, loading: true, errorCode: null } }));
      try {
        const data = await client.getHealth();
        set({ health: { data, loading: false, errorCode: null } });
      } catch {
        set((state) => ({ health: { ...state.health, loading: false, errorCode: "HEALTH_LOAD_FAILED" } }));
      }
    },
  };
}
