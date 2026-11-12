import React from "react";
import ReactDOM from "react-dom/client";
import { I18nextProvider } from "react-i18next";
import { createI18n } from "@localcamera/shared/i18n";
import { resolveLanguage, savedLanguage } from "./app/lang";
import "@fontsource-variable/inter";
import "@fontsource-variable/jetbrains-mono";
import "./index.css";
import App from "./App";

async function bootstrap() {
  const i18n = await createI18n(resolveLanguage(savedLanguage(), navigator.language));
  document.documentElement.classList.add("dark");
  ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
    <React.StrictMode>
      <I18nextProvider i18n={i18n}>
        <App />
      </I18nextProvider>
    </React.StrictMode>,
  );
}

void bootstrap();
