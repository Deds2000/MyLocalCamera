import type { AppGet, AppSet } from "./types";

export type UserRotation = 0 | 90 | 180 | 270;

export interface UiSnapshot {
  activeTab: string;
  fullscreen: boolean;
  dialogs: Record<string, boolean>;
  mirror: boolean;
  rotation: UserRotation;
}

export interface UiSlice {
  ui: UiSnapshot;
  setActiveTab: (tab: string) => void;
  setFullscreen: (fullscreen: boolean) => void;
  setDialogOpen: (dialog: string, open: boolean) => void;
  setMirror: (mirror: boolean) => void;
  setRotation: (rotation: UserRotation) => void;
}

export function createUiSlice(set: AppSet, get: AppGet, initialTab: string): UiSlice {
  return {
    ui: { activeTab: initialTab, fullscreen: false, dialogs: {}, mirror: false, rotation: 0 },
    setActiveTab: (activeTab) => {
      set((state) => ({ ui: { ...state.ui, activeTab } }));
      get().setActiveTabPreference(activeTab);
    },
    setFullscreen: (fullscreen) => set((state) => ({ ui: { ...state.ui, fullscreen } })),
    setDialogOpen: (dialog, open) => set((state) => ({
      ui: { ...state.ui, dialogs: { ...state.ui.dialogs, [dialog]: open } },
    })),
    setMirror: (mirror) => set((state) => ({ ui: { ...state.ui, mirror } })),
    setRotation: (rotation) => set((state) => ({ ui: { ...state.ui, rotation } })),
  };
}
