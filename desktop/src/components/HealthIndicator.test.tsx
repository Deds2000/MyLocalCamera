import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { HealthIndicator, type HealthState } from "./HealthIndicator";

describe("HealthIndicator (F3-T02)", () => {
  const states: HealthState[] = ["ok", "warn", "danger", "muted", "accent"];

  it("renderiza ponto + rótulo nos 5 estados", () => {
    for (const state of states) {
      const { unmount } = render(
        <HealthIndicator kind="server" state={state} detail={`:48651-${state}`} />,
      );
      expect(screen.getByText(`:48651-${state}`)).toBeInTheDocument();
      unmount();
    }
  });

  it("aplica a cor do ponto por estado", () => {
    const { container, unmount } = render(
      <HealthIndicator kind="server" state="ok" detail="ok" />,
    );
    expect(container.querySelector(".bg-ok")).not.toBeNull();
    unmount();
    const c2 = render(<HealthIndicator kind="vcam" state="danger" detail="x" />).container;
    expect(c2.querySelector(".bg-danger")).not.toBeNull();
  });

  it("com onClick vira botão e dispara a correção", async () => {
    const onClick = vi.fn();
    render(<HealthIndicator kind="adb" state="muted" detail="ADB" onClick={onClick} />);
    await userEvent.click(screen.getByRole("button"));
    expect(onClick).toHaveBeenCalledTimes(1);
  });
});
