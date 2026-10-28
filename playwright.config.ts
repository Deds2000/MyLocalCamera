import { defineConfig, devices } from "@playwright/test";

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
      command: "pnpm --filter desktop exec vite --port 1420",
      url: "http://localhost:1420",
      reuseExistingServer: true,
      timeout: 120_000,
    },
    {
      command: "pnpm --filter mobile exec vite --port 5173",
      url: "http://localhost:5173",
      reuseExistingServer: true,
      timeout: 120_000,
    },
  ],
});
