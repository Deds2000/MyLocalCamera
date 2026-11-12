import { screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { renderWithProviders } from "@localcamera/shared/test";
import { describe, expect, it, vi } from "vitest";
import { ApprovalDialog } from "./ApprovalDialog";

describe("ApprovalDialog (esqueleto, F3-T02)", () => {
  const props = {
    request: { name: "Pixel 8", model: "Pixel 8", ip: "192.168.0.23" },
    seconds: 27,
    onReject: vi.fn(),
    onApproveOnce: vi.fn(),
    onApproveAlways: vi.fn(),
  };

  it("mostra escudo, título com nome, modelo/IP e contagem (pt-BR)", async () => {
    await renderWithProviders(<ApprovalDialog {...props} />);

    expect(screen.getByTestId("approval-shield")).toBeInTheDocument();
    expect(screen.getByTestId("approval-title")).toHaveTextContent(
      'Permitir que "Pixel 8" se conecte?',
    );
    expect(screen.getByTestId("approval-body")).toHaveTextContent(
      "Modelo Pixel 8 · IP 192.168.0.23",
    );
    expect(screen.getByTestId("approval-timer")).toHaveTextContent("Expira em 27 s");
  });

  it("tem 3 botões e cada um chama o callback certo", async () => {
    const user = userEvent.setup();
    await renderWithProviders(<ApprovalDialog {...props} />);

    await user.click(screen.getByTestId("approval-reject"));
    expect(props.onReject).toHaveBeenCalledTimes(1);
    expect(props.onApproveOnce).not.toHaveBeenCalled();

    await user.click(screen.getByTestId("approval-once"));
    expect(props.onApproveOnce).toHaveBeenCalledTimes(1);

    await user.click(screen.getByTestId("approval-always"));
    expect(props.onApproveAlways).toHaveBeenCalledTimes(1);
  });

  it("traduz para en", async () => {
    await renderWithProviders(<ApprovalDialog {...props} />, "en");
    expect(screen.getByTestId("approval-title")).toHaveTextContent(
      'Allow "Pixel 8" to connect?',
    );
    expect(screen.getByTestId("approval-timer")).toHaveTextContent(
      "Expires in 27 s",
    );
    expect(screen.getByTestId("approval-always")).toHaveTextContent("Always allow");
  });
});
