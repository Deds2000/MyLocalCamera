import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { PreviewFrame } from "./PreviewFrame";

describe("PreviewFrame (F3-T02)", () => {
  it("mantém 16:9, raio xl, borda glass e letterbox raised", () => {
    render(<PreviewFrame />);
    const frame = screen.getByTestId("preview-frame");
    expect(frame).toHaveClass(
      "aspect-video",
      "rounded-xl",
      "border-glass-border",
      "bg-raised",
      "shadow-glass",
    );
  });

  it("renderiza children e overlay com fundo sólido (sem backdrop-filter)", () => {
    render(
      <PreviewFrame overlay={<span data-testid="ov">overlay</span>}>
        <span data-testid="vid">video</span>
      </PreviewFrame>,
    );
    expect(screen.getByTestId("vid")).toBeInTheDocument();
    const overlay = screen.getByTestId("ov").parentElement;
    expect(overlay).toHaveClass("bg-black/60");
    expect(overlay).not.toHaveClass("backdrop-blur");
  });
});
