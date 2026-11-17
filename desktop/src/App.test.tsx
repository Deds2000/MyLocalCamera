import { screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { renderWithProviders } from "@localcamera/shared/test";
import { beforeEach, describe, expect, it } from "vitest";
import App from "./App";
import { panelWidth, previewPadding } from "./app/layout";
import { appStore } from "./store";

function setWindowWidth(width: number) {
  Object.defineProperty(window, "innerWidth", { configurable: true, writable: true, value: width });
  window.dispatchEvent(new Event("resize"));
}

beforeEach(() => {
  localStorage.clear();
  appStore.setState({ settings: { ...appStore.getState().settings, language: "system" } });
  setWindowWidth(1024);
});

describe("App (desktop) — shell (F3-T01)", () => {
  it("renderiza titlebar, pílula, preview, painel e rodapé (pt-BR)", async () => {
    await renderWithProviders(<App />);

    expect(screen.getByTestId("titlebar")).toBeInTheDocument();
    expect(screen.getByTestId("status-pill")).toHaveTextContent("Desconectado");
    expect(screen.getByTestId("preview")).toBeInTheDocument();
    expect(screen.getByTestId("preview-frame")).toHaveClass("aspect-video");
    expect(screen.getByTestId("panel")).toBeInTheDocument();
    expect(screen.getByTestId("footer")).toBeInTheDocument();

    // wordmark: "Local" (text) + "Camera" (brand)
    const spans = screen.getAllByText(/^(Local|Camera)$/);
    expect(spans).toHaveLength(2);
    expect(spans[1]).toHaveClass("text-brand");

    // botões de janela
    expect(screen.getByRole("button", { name: "Minimizar" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Maximizar" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Fechar" })).toBeInTheDocument();
  });

  it("aplica breakpoint do painel conforme a largura da janela (resize)", async () => {
    const { unmount } = await renderWithProviders(<App />);
    const panel = screen.getByTestId("panel");
    expect(panel).toHaveStyle({ width: "300px" });

    setWindowWidth(1150);
    await waitFor(() => expect(panel).toHaveStyle({ width: "340px" }));

    setWindowWidth(1280);
    await waitFor(() => expect(panel).toHaveStyle({ width: "380px" }));
    unmount();
  });

  it("troca pt-BR ↔ en em tempo real e persiste a preferência", async () => {
    const user = userEvent.setup();
    const { unmount } = await renderWithProviders(<App />);

    // escolha inicial: "Sistema"
    await user.click(screen.getByRole("button", { name: "Sistema" }));
    const menu = screen.getByRole("menu");
    expect(menu).toBeInTheDocument();
    await user.click(screen.getByRole("menuitem", { name: "English" }));

    // textos visíveis mudam na hora
    await waitFor(() =>
      expect(screen.getByTestId("status-pill")).toHaveTextContent("Disconnected"),
    );
    expect(await screen.findByRole("button", { name: "English" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Maximize" })).toBeInTheDocument();
    expect(localStorage.getItem("lang")).toBe("en");

    unmount();

    // relançamento: o bootstrap (main.tsx) lê a preferência salva
    await renderWithProviders(<App />, "en");
    expect(await screen.findByRole("button", { name: "English" })).toBeInTheDocument();
    expect(screen.getByTestId("status-pill")).toHaveTextContent("Disconnected");
  });

  it("marca o idioma ativo no menu", async () => {
    const user = userEvent.setup();
    await renderWithProviders(<App />);

    await user.click(screen.getByRole("button", { name: "Sistema" }));
    const systemItem = screen.getByRole("menuitem", { name: "Sistema" });
    expect(systemItem.querySelector("svg")).toBeInTheDocument();

    await user.click(screen.getByRole("menuitem", { name: "Português" }));
    await waitFor(() =>
      expect(screen.getByTestId("status-pill")).toHaveTextContent("Desconectado"),
    );
    expect(localStorage.getItem("lang")).toBe("pt-BR");
  });
});

describe("panelWidth / previewPadding (UT-3.14)", () => {
  it("1280 → 380, 1100 → 340, 960 → 300", () => {
    expect(panelWidth(1280)).toBe(380);
    expect(panelWidth(1100)).toBe(340);
    expect(panelWidth(960)).toBe(300);
  });

  it("limites: 1279 → 340, 1099 → 300; margem 24 (≥1100) / 16 (abaixo)", () => {
    expect(panelWidth(1279)).toBe(340);
    expect(panelWidth(1099)).toBe(300);
    expect(previewPadding(1100)).toBe(24);
    expect(previewPadding(1099)).toBe(16);
  });
});
