import type { LanguageChoice } from "@/app/lang";
import type { AppSet } from "./types";

const LANGUAGE_KEY = "lang";
const ACTIVE_TAB_KEY = "active-tab";
const NETWORK_INTERFACE_KEY = "network-interface";

export interface SettingsSnapshot {
  language: LanguageChoice;
  activeTab: string;
  networkInterface: string | null;
}

export interface SettingsSlice {
  settings: SettingsSnapshot;
  setLanguage: (language: LanguageChoice) => void;
  setActiveTabPreference: (activeTab: string) => void;
  setNetworkInterface: (networkInterface: string | null) => void;
}

function readPreference(key: string): string | null {
  try {
    return localStorage.getItem(key);
  } catch {
    // Storage can be unavailable; keep the in-memory default.
  }
  return null;
}

function persistPreference(key: string, value: string | null): void {
  try {
    if (value === null) localStorage.removeItem(key);
    else localStorage.setItem(key, value);
  } catch {
    // Keep the in-memory preference if persistent storage is unavailable.
  }
}

export function createSettingsSlice(set: AppSet): SettingsSlice {
  const savedLanguage = readPreference(LANGUAGE_KEY);
  const language: LanguageChoice =
    savedLanguage === "pt-BR" || savedLanguage === "en" ? savedLanguage : "system";
  return {
    settings: {
      language,
      activeTab: readPreference(ACTIVE_TAB_KEY) ?? "connection",
      networkInterface: readPreference(NETWORK_INTERFACE_KEY),
    },
    setLanguage: (language) => {
      set((state) => ({ settings: { ...state.settings, language } }));
      persistPreference(LANGUAGE_KEY, language);
    },
    setActiveTabPreference: (activeTab) => {
      set((state) => ({ settings: { ...state.settings, activeTab } }));
      persistPreference(ACTIVE_TAB_KEY, activeTab);
    },
    setNetworkInterface: (networkInterface) => {
      set((state) => ({ settings: { ...state.settings, networkInterface } }));
      persistPreference(NETWORK_INTERFACE_KEY, networkInterface);
    },
  };
}
