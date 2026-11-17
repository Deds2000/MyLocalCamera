import { detectLanguage, type AppLanguage } from "@localcamera/shared/i18n";

export type LanguageChoice = "system" | AppLanguage;

export function resolveLanguage(choice: LanguageChoice, systemLang?: string): AppLanguage {
  return choice === "system" ? detectLanguage(systemLang ?? navigator.language) : choice;
}
