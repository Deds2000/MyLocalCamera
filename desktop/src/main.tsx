import React from "react";
import ReactDOM from "react-dom/client";
import { createI18n, detectLanguage } from "@localcamera/shared/i18n";
import "@fontsource-variable/inter";
import "@fontsource-variable/jetbrains-mono";
import "./index.css";
import App from "./App";

async function bootstrap() {
  await createI18n(detectLanguage(navigator.language));
  document.documentElement.classList.add("dark");
  ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
    <React.StrictMode>
      <App />
    </React.StrictMode>,
  );
}

void bootstrap();
