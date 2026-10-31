import { describe, expect, it } from "vitest";
import presetDefault, { tokens } from "../tailwind-preset";

// o tipo `Config` do Tailwind aceita funções/resolvers; para o teste basta o objeto simples
const preset = presetDefault as {
  theme?: {
    extend?: {
      boxShadow?: Record<string, string>;
      borderRadius?: Record<string, string>;
    };
  };
};

describe("UT-0.02 tailwind-preset", () => {
  it("tokens do design system", () => {
    expect(tokens.accent).toBe("#22D3EE");
    expect(tokens.bg).toBe("#070B14");
    expect(tokens.danger).toBe("#FB7185");
  });

  it("preset: theme.extend.boxShadow tem glass, overlay e glow-accent", () => {
    const boxShadow = preset.theme?.extend?.boxShadow ?? {};
    expect(Object.keys(boxShadow).sort()).toEqual(["glass", "glow-accent", "overlay"]);
  });

  it("preset: borderRadius.xl === '24px'", () => {
    expect(preset.theme?.extend?.borderRadius?.xl).toBe("24px");
  });
});
