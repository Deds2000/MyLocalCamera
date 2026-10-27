import { describe, expect, it } from "vitest";
import { cn } from "./cn";

describe("UT-0.01 cn", () => {
  it("conflieto de classe: a última vence", () => {
    expect(cn("p-2", "p-4")).toBe("p-4");
  });

  it("ignora valores falsos e junta as demais", () => {
    // via parâmetro para não tocar em `no-constant-binary-expression`
    const withFlag = (cond: boolean) => cn("a", cond && "b", "c");
    expect(withFlag(false)).toBe("a c");
  });

  it("cor: a última vence", () => {
    expect(cn("text-red-500", "text-blue-500")).toBe("text-blue-500");
  });
});
