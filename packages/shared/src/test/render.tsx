import type { ReactElement } from "react";
import { render } from "@testing-library/react";
import { I18nextProvider } from "react-i18next";
import { createI18n, type AppLanguage } from "../i18n";

export async function renderWithProviders(ui: ReactElement, lng: AppLanguage = "pt-BR") {
  const i18n = await createI18n(lng);
  return render(<I18nextProvider i18n={i18n}>{ui}</I18nextProvider>);
}
