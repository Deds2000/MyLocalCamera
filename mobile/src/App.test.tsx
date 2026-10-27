import { screen } from "@testing-library/react";
import { renderWithProviders } from "@localcamera/shared/test";
import { describe, expect, it } from "vitest";
import App from "./App";

describe("App (mobile) smoke", () => {
  it("mostra o botão 'Abrir' (pt-BR)", async () => {
    await renderWithProviders(<App />);
    const button = await screen.findByRole("button", { name: "Abrir" });
    expect(button).toBeInTheDocument();
  });
});
