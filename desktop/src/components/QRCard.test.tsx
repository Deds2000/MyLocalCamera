import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { QRCard } from "./QRCard";

describe("QRCard (F3-T02)", () => {
  const props = {
    uri: "localcamera://pair?token=abc123",
    hosts: ["192.168.0.10", "PC-DESKTOP"],
    port: 48651,
  };

  it("renderiza o QR (220 px) dentro da margem clara de 16 px", () => {
    const { container } = render(<QRCard {...props} />);
    const qr = container.querySelector("svg");
    expect(qr).not.toBeNull();
    // QR de 220 px
    expect(qr?.getAttribute("width")).toBe("220");
    // margem clara: 16 px (p-4) de fundo branco ao redor
    const margin = screen.getByTestId("qr-margin");
    expect(margin).toHaveClass("bg-white", "p-4");
  });

  it("exibe hosts e porta sob o QR", () => {
    render(<QRCard {...props} />);
    expect(screen.getByTestId("qr-hosts")).toHaveTextContent("192.168.0.10 · PC-DESKTOP");
    expect(screen.getByTestId("qr-port")).toHaveTextContent("48651");
  });
});
