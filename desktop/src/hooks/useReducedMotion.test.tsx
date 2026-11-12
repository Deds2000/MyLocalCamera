import { act, renderHook } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { cardVariants, dialogVariants, sectionVariants } from "../lib/motion";
import { useReducedMotion } from "./useReducedMotion";

type Listener = (event: MediaQueryListEvent) => void;

function mockMatchMedia(initialMatches: boolean) {
  let current = initialMatches;
  const listeners = new Set<Listener>();
  const mql = {
    get matches() {
      return current;
    },
    media: "(prefers-reduced-motion: reduce)",
    onchange: null,
    addEventListener: vi.fn((_type: string, fn: Listener) => listeners.add(fn)),
    removeEventListener: vi.fn((_type: string, fn: Listener) => listeners.delete(fn)),
    dispatchEvent: vi.fn(),
  };
  vi.stubGlobal("matchMedia", vi.fn().mockReturnValue(mql));
  return {
    mql,
    fire(matchesValue: boolean) {
      current = matchesValue;
      for (const fn of [...listeners]) {
        fn({ matches: matchesValue } as unknown as MediaQueryListEvent);
      }
    },
  };
}

describe("useReducedMotion (F3-T02)", () => {
  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it("é false quando a media query não está ativa", () => {
    mockMatchMedia(false);
    const { result } = renderHook(() => useReducedMotion());
    expect(result.current).toBe(false);
  });

  it("é true quando prefers-reduced-motion: reduce está ativa", () => {
    mockMatchMedia(true);
    const { result } = renderHook(() => useReducedMotion());
    expect(result.current).toBe(true);
  });

  it("reage a mudanças da media query", () => {
    const { fire } = mockMatchMedia(false);
    const { result } = renderHook(() => useReducedMotion());
    expect(result.current).toBe(false);
    act(() => fire(true));
    expect(result.current).toBe(true);
    act(() => fire(false));
    expect(result.current).toBe(false);
  });
});

describe("variantes de animação x prefers-reduced-motion (F3-T02)", () => {
  it("reduzido: só fade, sem deslocamento, ≤ 80 ms", () => {
    for (const make of [sectionVariants, cardVariants, dialogVariants]) {
      const variants = make(true);
      for (const state of ["initial", "animate", "exit"]) {
        const entry = variants[state] as Record<string, unknown>;
        expect(entry.x, `${state}.x`).toBeUndefined();
        expect(entry.y, `${state}.y`).toBeUndefined();
        expect(entry.scale, `${state}.scale`).toBeUndefined();
        expect(entry.opacity, `${state}.opacity`).toBe(state === "animate" ? 1 : 0);
        const transition = entry.transition as { duration?: number } | undefined;
        if (transition?.duration !== undefined) {
          expect(transition.duration, `${state}.duration`).toBeLessThanOrEqual(0.08);
        }
      }
    }
  });

  it("normal: mantém fade + 8 px (seção) e scale (cartão 0.98, diálogo 0.96)", () => {
    const section = sectionVariants(false) as Record<string, Record<string, unknown>>;
    expect(section.initial).toMatchObject({ opacity: 0, y: 8 });
    expect(section.animate).toMatchObject({ opacity: 1, y: 0 });

    const card = cardVariants(false) as Record<string, Record<string, unknown>>;
    expect(card.initial).toMatchObject({ opacity: 0, scale: 0.98 });
    expect(card.animate).toMatchObject({ opacity: 1, scale: 1 });

    const dialog = dialogVariants(false) as Record<string, Record<string, unknown>>;
    expect(dialog.initial).toMatchObject({ opacity: 0, scale: 0.96 });
    expect(dialog.animate).toMatchObject({ opacity: 1, scale: 1 });
  });
});
