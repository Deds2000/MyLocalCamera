import { create } from "zustand";
import { createStore, type StateCreator, type StoreApi } from "zustand/vanilla";
import { tauri } from "@/lib/tauri";
import { createCameraSlice } from "./camera";
import { createConnectionSlice } from "./connection";
import { createHealthSlice } from "./health";
import { createSettingsSlice } from "./settings";
import { createStatsSlice } from "./stats";
import { createTrustSlice } from "./trust";
import { createUiSlice } from "./ui";
import { createUsbSlice } from "./usb";
import type { AppStore } from "./types";
import type { TauriClient } from "@/lib/tauri";

function appState(client: TauriClient): StateCreator<AppStore> {
  return (set, get) => {
    const settings = createSettingsSlice(set);
    return {
      ...createConnectionSlice(set, get, client),
      ...createCameraSlice(set),
      ...createStatsSlice(set),
      ...createTrustSlice(set, get, client),
      ...createUsbSlice(),
      ...createHealthSlice(set, client),
      ...settings,
      ...createUiSlice(set, get, settings.settings.activeTab),
    };
  };
}

/** Isolated store factory used by tests; the app uses the singleton below. */
export function createAppStore(client: TauriClient = tauri): StoreApi<AppStore> {
  return createStore<AppStore>()(appState(client));
}

export const useAppStore = create<AppStore>()(appState(tauri));
export const appStore: StoreApi<AppStore> = useAppStore;

export const selectConnection = (state: AppStore) => state.connection;
export const selectConnectionState = (state: AppStore) => state.connection.state;
export const selectCameraState = (state: AppStore) => state.cameraState;
export const selectStats = (state: AppStore) => state.stats;
export const selectTrustPending = (state: AppStore) => state.trust.pending;
export const selectTrustedDevices = (state: AppStore) => state.trust.trusted;
export const selectUsbDevices = (state: AppStore) => state.usb.devices;
export const selectHealth = (state: AppStore) => state.health;
export const selectHealthData = (state: AppStore) => state.health.data;
export const selectHealthRestarting = (state: AppStore) => state.health.restarting;
export const selectLanguage = (state: AppStore) => state.settings.language;
export const selectActiveTab = (state: AppStore) => state.ui.activeTab;
export const selectUi = (state: AppStore) => state.ui;

export type { AppStore } from "./types";
export type { CameraStateEvent } from "./camera";
export type { DesktopConnectionState } from "./connection";
