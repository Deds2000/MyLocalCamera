import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import {
  DeviceRow,
  EmptyState,
  LevelMeter,
  SegmentedControl,
  SettingRow,
  StatTile,
  StatusChip,
  type ConnectionState,
} from "./index";

describe("StatusChip (shared)", () => {
  const states: ConnectionState[] = [
    "idle",
    "connecting",
    "pending",
    "streaming",
    "reconnecting",
    "error",
  ];

  it("renderiza nos 6 estados sem erro", () => {
    for (const state of states) {
      const { unmount } = render(<StatusChip state={state} label={`L-${state}`} />);
      expect(screen.getByText(`L-${state}`)).toBeInTheDocument();
      unmount();
    }
  });

  it("aplica a cor do estado (streaming = ok, error = danger, connecting = brand)", () => {
    const { container, unmount } = render(<StatusChip state="streaming" label="ok" />);
    expect(container.querySelector(".text-ok")).not.toBeNull();
    unmount();
    const c2 = render(<StatusChip state="error" label="err" />).container;
    expect(c2.querySelector(".text-danger")).not.toBeNull();
    const c3 = render(<StatusChip state="connecting" label="con" />).container;
    expect(c3.querySelector(".text-brand")).not.toBeNull();
  });

  it("aceita ícone custom no lugar do ícone do estado", () => {
    render(
      <StatusChip
        state="idle"
        label="custom"
        icon={<span data-testid="custom-icon">i</span>}
      />,
    );
    expect(screen.getByTestId("custom-icon")).toBeInTheDocument();
  });
});

describe("StatTile (shared)", () => {
  it("renderiza label, valor mono/tabular, unidade e hint", () => {
    render(<StatTile label="Bitrate" value="8.4" unit="Mbps" hint="auto" />);
    expect(screen.getByText("Bitrate")).toBeInTheDocument();
    expect(screen.getByText("8.4")).toHaveClass("font-mono", "tabular-nums");
    expect(screen.getByText("Mbps")).toBeInTheDocument();
    expect(screen.getByText("auto")).toBeInTheDocument();
  });

  it("omite unidade e hint quando ausentes", () => {
    render(<StatTile label="FPS" value={30} />);
    expect(screen.getByText("30")).toBeInTheDocument();
    expect(screen.queryByText("Mbps")).not.toBeInTheDocument();
  });
});

describe("SegmentedControl (shared)", () => {
  const options = [
    { value: "a", label: "Op A" },
    { value: "b", label: "Op B" },
    { value: "c", label: "Op C" },
  ];

  it("marca a opção selecionada e chama onChange ao clicar", async () => {
    const onChange = vi.fn();
    render(
      <SegmentedControl
        options={options}
        value="a"
        onChange={onChange}
        ariaLabel="Grupo"
      />,
    );
    expect(screen.getByRole("radio", { name: "Op A" })).toHaveAttribute(
      "aria-checked",
      "true",
    );
    await userEvent.click(screen.getByRole("radio", { name: "Op B" }));
    expect(onChange).toHaveBeenCalledWith("b");
  });

  it("desabilita opção com motivo (tooltip) e não chama onChange", async () => {
    const onChange = vi.fn();
    render(
      <SegmentedControl
        options={options}
        value="a"
        onChange={onChange}
        disabledReasons={{ c: "Sem permissão" }}
      />,
    );
    const disabled = screen.getByRole("radio", { name: "Op C" });
    expect(disabled).toBeDisabled();
    expect(disabled).toHaveAttribute("title", "Sem permissão");
    await userEvent.click(disabled);
    expect(onChange).not.toHaveBeenCalled();
  });
});

describe("SettingRow (shared)", () => {
  it("renderiza label, descrição e controle", () => {
    render(
      <SettingRow
        label="Qualidade"
        description="Alta, média ou baixa"
        control={<button type="button">Ctrl</button>}
      />,
    );
    expect(screen.getByText("Qualidade")).toBeInTheDocument();
    expect(screen.getByText("Alta, média ou baixa")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Ctrl" })).toBeInTheDocument();
  });

  it("mostra o ícone sutil do celular quando appliesTo é phone/both", () => {
    const { container: phone } = render(
      <SettingRow label="X" control={<span />} appliesTo="phone" />,
    );
    expect(phone.querySelector("svg")).not.toBeNull();
    const { container: pc } = render(
      <SettingRow label="Y" control={<span />} appliesTo="pc" />,
    );
    expect(pc.querySelector("svg")).toBeNull();
    const { container: both } = render(
      <SettingRow label="Z" control={<span />} appliesTo="both" />,
    );
    expect(both.querySelector("svg")).not.toBeNull();
  });
});

describe("DeviceRow (shared)", () => {
  it("renderiza icon, title, subtitle, badge e action", () => {
    render(
      <DeviceRow
        icon={<span data-testid="d-icon">i</span>}
        title="Pixel 8"
        subtitle="192.168.0.23"
        badge={<span data-testid="d-badge">b</span>}
        action={<button type="button">Ação</button>}
      />,
    );
    expect(screen.getByTestId("d-icon")).toBeInTheDocument();
    expect(screen.getByText("Pixel 8")).toBeInTheDocument();
    expect(screen.getByText("192.168.0.23")).toBeInTheDocument();
    expect(screen.getByTestId("d-badge")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Ação" })).toBeInTheDocument();
  });
});

describe("EmptyState (shared)", () => {
  it("renderiza icon, title, description e actions", () => {
    render(
      <EmptyState
        icon={<span data-testid="e-icon">i</span>}
        title="Vazio"
        description="Nada por aqui"
        actions={<button type="button">Começar</button>}
      />,
    );
    expect(screen.getByTestId("e-icon")).toBeInTheDocument();
    expect(screen.getByText("Vazio")).toBeInTheDocument();
    expect(screen.getByText("Nada por aqui")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Começar" })).toBeInTheDocument();
  });
});

describe("LevelMeter (shared)", () => {
  it("usa transform e fica cinza quando desligado", () => {
    const { container, rerender } = render(<LevelMeter level={0} peak={0.4} />);
    const fill = screen.getByTestId("level-fill");
    const peak = screen.getByTestId("level-peak");
    expect(fill).toHaveClass("bg-text-muted");
    expect(fill.style.transform).toBe("scaleX(0)");
    expect(peak.style.transform).toBe("translateX(40%)");

    rerender(<LevelMeter level={0.5} peak={0.8} />);
    expect(fill).toHaveClass("bg-ok");
    expect(fill.style.transform).toBe("scaleX(0.5)");
    expect(screen.getByTestId("level-meter")).toBe(container.firstChild);
  });

  it("satura valores acima de 1 e abaixo de 0", () => {
    render(<LevelMeter level={2} peak={-1} />);
    expect(screen.getByTestId("level-fill").style.transform).toBe("scaleX(1)");
    expect(screen.getByTestId("level-peak").style.transform).toBe(
      "translateX(0%)",
    );
  });
});
