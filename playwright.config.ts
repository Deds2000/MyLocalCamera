import { defineConfig, devices } from "@playwright/test";

// Portas padrão isoladas das aplicações locais comuns (Vite mobile usa 5173).
// Para ambientes compartilhados, escolha duas portas TCP livres e defina
// LC_E2E_DESKTOP_PORT / LC_E2E_MOBILE_PORT; configuração e specs leem as mesmas.
export const e2ePorts = {
  desktop: Number(process.env.LC_E2E_DESKTOP_PORT ?? 1421),
  mobile: Number(process.env.LC_E2E_MOBILE_PORT ?? 5199),
};

// Regressão visual dos apps no navegador (sem Tauri/Capacitor).
// O catálogo real (?catalog) chega nas Fases 2/3; por ora só a home.
export default defineConfig({
  testDir: "./e2e",
  // snapshotDir ficou descontinuada; template equivalente mantendo e2e/__snapshots__
  snapshotPathTemplate: "e2e/__snapshots__/{projectName}/{arg}{ext}",
  expect: {
    timeout: 15_000,
  },
  reporter: [["list"]],
  projects: [
    {
      name: "desktop-960x640",
      use: {
        ...devices["Desktop Chrome"],
        viewport: { width: 960, height: 640 },
      },
    },
    {
      name: "desktop-1280x800",
      use: {
        ...devices["Desktop Chrome"],
        viewport: { width: 1280, height: 800 },
      },
    },
    {
      name: "mobile",
      use: {
        ...devices["iPhone 13"],
        browserName: "chromium",
        viewport: { width: 390, height: 844 },
      },
    },
  ],
  webServer: [
    {
      command: `pnpm --filter desktop exec vite --host 127.0.0.1 --strictPort --port ${e2ePorts.desktop}`,
      url: `http://127.0.0.1:${e2ePorts.desktop}`,
      reuseExistingServer: false,
      timeout: 120_000,
    },
    {
      command: `pnpm --filter mobile exec vite --host 127.0.0.1 --strictPort --port ${e2ePorts.mobile}`,
      url: `http://127.0.0.1:${e2ePorts.mobile}`,
      reuseExistingServer: false,
      timeout: 120_000,
    },
  ],
});
