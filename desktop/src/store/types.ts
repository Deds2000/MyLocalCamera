import type { StoreApi } from "zustand";
import type { CameraSlice } from "./camera";
import type { ConnectionSlice } from "./connection";
import type { HealthSlice } from "./health";
import type { SettingsSlice } from "./settings";
import type { StatsSlice } from "./stats";
import type { TrustSlice } from "./trust";
import type { UiSlice } from "./ui";
import type { UsbSlice } from "./usb";

export type AppStore = ConnectionSlice &
  CameraSlice &
  StatsSlice &
  TrustSlice &
  UsbSlice &
  HealthSlice &
  SettingsSlice &
  UiSlice;

export type AppSet = StoreApi<AppStore>["setState"];
export type AppGet = StoreApi<AppStore>["getState"];
