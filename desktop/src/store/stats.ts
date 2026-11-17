import type { StatsEvent } from "@/lib/tauri";
import type { AppSet } from "./types";

export interface StatsSlice {
  stats: StatsEvent | null;
  applyStatsEvent: (event: StatsEvent) => void;
}

export function createStatsSlice(set: AppSet): StatsSlice {
  return {
    stats: null,
    applyStatsEvent: (event) => set({ stats: { ...event } }),
  };
}
