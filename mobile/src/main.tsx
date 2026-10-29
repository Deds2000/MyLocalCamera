import React from "react";
import ReactDOM from "react-dom/client";
import { I18nextProvider } from "react-i18next";
import { App as CapApp } from "@capacitor/app";
import { createI18n, detectLanguage } from "@localcamera/shared/i18n";
import "@fontsource-variable/inter";
import "@fontsource-variable/jetbrains-mono";
import "./index.css";
import App from "./App";
import { setPendingDeepLink } from "./deep-link";

// Deep link `localcamera://connect` (F2-T01): registrado antes de o React
// montar, para não perder `appUrlOpen` durante o bootstrap.
void CapApp.addListener("appUrlOpen", (event) => {
  setPendingDeepLink(event.url);
});

async function bootstrap() {
  const i18n = await createI18n(detectLanguage(navigator.language));
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
