import path from "node:path";
import { defineConfig } from "vitest/config";
import react from "@vitejs/plugin-react";

export default defineConfig({
  plugins: [react()],
  resolve: {
    // F3-T14b: pseudo.test.tsx renderiza o shell do desktop (App da F3-T01).
    // O resolver do Vite não alcança caminhos fora do package; o alias aponta
    // diretamente para o arquivo (apenas para o teste, não para produção).
    alias: [
      {
        find: /^@\//,
        replacement: `${path.resolve(import.meta.dirname, "../../desktop/src")}/`,
      },
      {
        find: "@localcamera/desktop-shell",
        replacement: path.resolve(import.meta.dirname, "../../desktop/src/app/Shell.tsx"),
      },
    ],
  },
  test: {
    environment: "jsdom",
    globals: true,
    setupFiles: ["./src/test/setup.ts"],
    css: false,
  },
});
