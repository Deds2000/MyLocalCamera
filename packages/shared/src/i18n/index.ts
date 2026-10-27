import i18n from "i18next";
import { initReactI18next } from "react-i18next";
import ptBR from "./pt-BR.json";
import en from "./en.json";

export const resources = { "pt-BR": { translation: ptBR }, en: { translation: en } } as const;
export type AppLanguage = "pt-BR" | "en";

export function detectLanguage(systemLang: string | undefined): AppLanguage {
  return systemLang?.toLowerCase().startsWith("pt") ? "pt-BR" : "en";
}

export async function createI18n(lng: AppLanguage = "pt-BR") {
  const instance = i18n.createInstance();
  await instance.use(initReactI18next).init({
    resources,
    lng,
    fallbackLng: "en",
    interpolation: { escapeValue: false },
  });
  return instance;
}
