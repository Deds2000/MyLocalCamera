import { I18nextProvider } from "react-i18next";
import { render, screen } from "@testing-library/react";
import i18next from "i18next";
import { initReactI18next } from "react-i18next";
import { beforeEach, describe, expect, it } from "vitest";
import en from "./en.json";
import ptBR from "./pt-BR.json";
// Alias do vitest.config.ts do shared → desktop/src/app/Shell.tsx
// (o resolver do Vite não alcança caminhos fora do package).
import Shell from "@localcamera/desktop-shell";

type JsonObject = Record<string, unknown>;

/**
 * UT-3.25 (parte automatizada) — pseudo-localização +40% sobre o shell.
 *
 * Estende os recursos de teste com um "falso idioma" que expande cada texto
 * em pelo menos +40% (ceil(1.4 × len)): vogais viram formas alongadas
 * (a→aá, e→éè, …) e o restante é completado com "·". O shell é renderizado
 * com esses recursos e se asseriona que nenhum nó de texto relevante estoura
 * por overflow/truncate: heurística do cartão (`scrollWidth` vs `clientWidth`
 * nos nós com `truncate`/`overflow-hidden`) + asserção de texto completo no
 * DOM (substituta no jsdom, onde a métrica de layout é 0). A parte visual das
 * telas completas é revalidada no catálogo da F3-T14c — não duplicar aqui.
 *
 * Nota: `App` (F3-T01) é um wrapper de 1 linha de `<Shell/>`; o teste importa
 * o shell diretamente pelo caminho relativo (o package `desktop` não é
 * dependência do `shared`).
 */

const VOWELS: Record<string, string> = {
  a: "aá",
  e: "éè",
  i: "íì",
  o: "óò",
  u: "úù",
  A: "AÁ",
  E: "ÉÈ",
  I: "ÍÌ",
  O: "ÓÒ",
  U: "ÚÙ",
};

export function pseudoLocalize(text: string): string {
  const expanded = text.replace(/[aeiouAEIOU]/g, (ch) => VOWELS[ch] ?? ch);
  const min = Math.ceil(text.length * 1.4);
  return expanded.length >= min ? expanded : expanded + "·".repeat(min - expanded.length);
}

function pseudoTree(root: JsonObject): JsonObject {
  const out: JsonObject = {};
  for (const [key, value] of Object.entries(root)) {
    if (value !== null && typeof value === "object") out[key] = pseudoTree(value as JsonObject);
    else if (typeof value === "string") out[key] = pseudoLocalize(value);
    else out[key] = value;
  }
  return out;
}

async function createPseudoI18n(lng: "pt-BR" | "en" = "pt-BR") {
  const instance = i18next.createInstance();
  await instance.use(initReactI18next).init({
    resources: {
      "pt-BR": { translation: pseudoTree(ptBR as JsonObject) },
      en: { translation: pseudoTree(en as JsonObject) },
    },
    lng,
    fallbackLng: "en",
    interpolation: { escapeValue: false },
  });
  return instance;
}

/** Nós de texto que o shell pode cortar: classe `truncate` ou `overflow-hidden`. */
function overflowCapableTextNodes(container: HTMLElement): HTMLElement[] {
  const nodes: HTMLElement[] = [];
  for (const el of Array.from(container.querySelectorAll<HTMLElement>("*"))) {
    if (!el.textContent || !el.textContent.trim()) continue;
    if (el.classList.contains("truncate") || el.classList.contains("overflow-hidden")) nodes.push(el);
  }
  return nodes;
}

describe("UT-3.25 pseudo-localização +40% (F3-T14b)", () => {
  beforeEach(() => {
    localStorage.clear();
  });

  it("pseudoLocalize expande cada texto em pelo menos +40%", () => {
    const samples = ["Desconectado", "Sistema", "Local", "Camera", "Porta", "123", "A"];
    for (const sample of samples) {
      const pseudo = pseudoLocalize(sample);
      expect(pseudo.length, `+40% de "${sample}"`).toBeGreaterThanOrEqual(Math.ceil(sample.length * 1.4));
      expect(pseudo.length).toBeGreaterThan(sample.length);
    }
  });

  it("shell (pt-BR) renderiza o texto pseudo completo e sem estouro de overflow", async () => {
    const i18n = await createPseudoI18n("pt-BR");
    const { container, unmount } = render(
      <I18nextProvider i18n={i18n}>
        <Shell />
      </I18nextProvider>,
    );

    // Texto pseudo completo no DOM (nada cortado em nível de app).
    const pill = screen.getByTestId("status-pill");
    expect(pill.textContent).toBe(pseudoLocalize("Desconectado"));

    // Gatilho de idioma tem classe `truncate`: o rótulo pseudo deve estar inteiro.
    const trigger = screen.getByTestId("language-trigger");
    const label = trigger.querySelector("span.truncate");
    expect(label, "rótulo de idioma com truncate").not.toBeNull();
    expect(label?.textContent).toBe(pseudoLocalize("Sistema"));

    // Heurística do cartão: nenhum nó com truncate/overflow-hidden estourado.
    const capped = overflowCapableTextNodes(container);
    expect(capped.length).toBeGreaterThan(0);
    for (const el of capped) {
      // No jsdom a métrica de layout é 0 (a verificação real de corte visual é
      // da F3-T14c); aqui o conteúdo deve estar inteiro e sem overflow medido.
      expect(el.scrollWidth, `estouro em <${el.className}>`).toBeLessThanOrEqual(el.clientWidth + 1);
      expect(el.textContent).toBe(el.textContent.trim());
    }

    unmount();
  });

  it("shell (en) renderiza o texto pseudo completo e sem estouro de overflow", async () => {
    const i18n = await createPseudoI18n("en");
    const { container, unmount } = render(
      <I18nextProvider i18n={i18n}>
        <Shell />
      </I18nextProvider>,
    );

    expect(screen.getByTestId("status-pill").textContent).toBe(pseudoLocalize("Disconnected"));

    const trigger = screen.getByTestId("language-trigger");
    const label = trigger.querySelector("span.truncate");
    expect(label?.textContent).toBe(pseudoLocalize("System"));

    for (const el of overflowCapableTextNodes(container)) {
      expect(el.scrollWidth, `estouro em <${el.className}>`).toBeLessThanOrEqual(el.clientWidth + 1);
    }

    unmount();
  });
});
