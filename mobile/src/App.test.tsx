import { screen } from "@testing-library/react";
import { renderWithProviders } from "@localcamera/shared/test";
import { describe, expect, it } from "vitest";
import App from "./App";
import { AppProvider } from "./state/AppProvider";
import { initialState, type AppState } from "./state/connection";

const allGranted: AppState = {
  ...initialState,
  permissions: { camera: "granted", mic: "granted", notifications: "granted" },
  permissionsDone: true,
};

describe("App (mobile) — roteamento derivado do estado", () => {
  it("mostra Permissões na primeira execução (câmera pendente)", async () => {
    await renderWithProviders(
      <AppProvider>
        <App />
      </AppProvider>,
    );
    expect(
      screen.getByRole("heading", { name: "Precisamos de algumas permissões" }),
    ).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Continuar" })).toBeDisabled();
  });

  it("mostra Home quando as permissões estão concedidas e concluídas", async () => {
    await renderWithProviders(
      <AppProvider initialState={allGranted}>
        <App />
      </AppProvider>,
    );
    expect(await screen.findByText("Pronto para conectar")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Escanear QR code" })).toBeInTheDocument();
  });
});
