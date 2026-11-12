/**
 * Preferência de idioma da titlebar (F3-T01).
 *
 * Persistida em `localStorage("lang")` — o slice `settings` da F3-T14a migra
 * esse valor para o store; não recriar a lógica aqui.
 */
import { detectLanguage, type AppLanguage } from "@localcamera/shared/i18n";

export type LanguageChoice = "system" | AppLanguage;

const STORAGE_KEY = "lang";

export function savedLanguage(): LanguageChoice {
  try {
    const value = localStorage.getItem(STORAGE_KEY);
    if (value === "pt-BR" || value === "en") return value;
    if (value === "system") return "system";
  } catch {
    // localStorage indisponível → padrão
  }
  return "system";
}

export function storeLanguage(choice: LanguageChoice): void {
  try {
    localStorage.setItem(STORAGE_KEY, choice);
  } catch {
    // sem persistência (modo privado) → segue apenas em memória
  }
}

export function resolveLanguage(
  choice: LanguageChoice,
  systemLang?: string,
): AppLanguage {
  return choice === "system" ? detectLanguage(systemLang ?? navigator.language) : choice;
}
