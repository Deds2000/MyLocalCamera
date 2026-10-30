// F2-T03c — Rotas do catálogo `?catalog` (dev) para:
// - UT-2.13: captura visual por componente/estado
// - UT-2.14: acessibilidade automática (axe) — sem violações sérias
//
// Criado aqui; a EXECUÇÃO acontece em F2-T14 (`pnpm e2e`, projeto `mobile`,
// chromium; dev server na porta 5173 com `import.meta.env.DEV=true`).
// Os baselines ficam em `e2e/__snapshots__/mobile/` (snapshotPathTemplate)
// e os PNGs finais são copiados para `docs/ui/screens/mobile/`.
//
// `reducedMotion: "reduce"` torna as capturas determinísticas: LiveRing
// (pulso) e StatusChip (spin) param de animar (os componentes já respeitam
// `prefers-reduced-motion`).

import { expect, test, type Page, type TestInfo } from "@playwright/test";

const CATALOG_URL = "http://localhost:5173/?catalog";

test.use({ reducedMotion: "reduce" });

/** O catálogo só existe no projeto `mobile` (F0-T10: mobile = chromium 390x844). */
function skipIfNotMobile(testInfo: TestInfo): string | false {
  return testInfo.project.name !== "mobile"
    ? "só roda no projeto mobile (chromium)"
    : false;
}

async function openCatalog(page: Page): Promise<void> {
  await page.goto(CATALOG_URL);
  await expect(page.locator("main h1")).toBeVisible();
}

async function capture(
  page: Page,
  testid: string,
  name: string,
  options?: { maxDiffPixelRatio?: number },
): Promise<void> {
  const el = page.locator(`[data-testid="${testid}"]`);
  await el.scrollIntoViewIfNeeded();
  await expect(el).toHaveScreenshot(name, {
    maxDiffPixelRatio: 0.01,
    ...options,
  });
}

test("UT-2.13 — StatusChip: 6 estados de conexão", async ({ page }, testInfo) => {
  test.skip(skipIfNotMobile(testInfo));
  await openCatalog(page);
  for (const state of [
    "idle",
    "connecting",
    "pending",
    "streaming",
    "reconnecting",
    "error",
  ]) {
    await capture(page, `catalog-statuschip-${state}`, `catalog-statuschip-${state}.png`);
  }
});

test("UT-2.13 — CountdownRing: 30/15/5 s", async ({ page }, testInfo) => {
  test.skip(skipIfNotMobile(testInfo));
  await openCatalog(page);
  for (const seconds of [30, 15, 5]) {
    await capture(
      page,
      `catalog-ring-${seconds}`,
      `catalog-countdownring-${seconds}s.png`,
    );
  }
});

test("UT-2.13 — BigToggle: ligado/desligado", async ({ page }, testInfo) => {
  test.skip(skipIfNotMobile(testInfo));
  await openCatalog(page);
  await capture(page, "catalog-bigtoggle-on", "catalog-bigtoggle-on.png");
  await capture(page, "catalog-bigtoggle-off", "catalog-bigtoggle-off.png");
});

test("UT-2.13 — LiveRing: ativo/pausado", async ({ page }, testInfo) => {
  test.skip(skipIfNotMobile(testInfo));
  await openCatalog(page);
  await capture(page, "catalog-liverring-active", "catalog-liverring-active.png");
  await capture(page, "catalog-liverring-paused", "catalog-liverring-paused.png");
});

test("UT-2.13 — StatTile: normal e rede lenta", async ({ page }, testInfo) => {
  test.skip(skipIfNotMobile(testInfo));
  await openCatalog(page);
  await capture(page, "catalog-stattile-normal", "catalog-stattile-normal.png");
  await capture(page, "catalog-stattile-slow", "catalog-stattile-slow.png");
});

test("UT-2.13 — SegmentedControl: seleção (padrão e alterada)", async ({
  page,
}, testInfo) => {
  test.skip(skipIfNotMobile(testInfo));
  await openCatalog(page);
  await capture(page, "catalog-segmented", "catalog-segmented-selected.png");
  await page
    .locator('[data-testid="catalog-segmented"] [role="radio"]')
    .nth(1)
    .click();
  await expect(
    page.locator('[data-testid="catalog-segmented"] [role="radio"]').nth(1),
  ).toHaveAttribute("aria-checked", "true");
  await capture(
    page,
    "catalog-segmented",
    "catalog-segmented-changed.png",
  );
});

test("UT-2.13 — Dialog (shadcn): aberto", async ({ page }, testInfo) => {
  test.skip(skipIfNotMobile(testInfo));
  await openCatalog(page);
  await page.locator('[data-testid="catalog-dialog-open"]').click();
  const dialog = page.getByRole("dialog");
  await expect(dialog).toBeVisible();
  await dialog.scrollIntoViewIfNeeded();
  await expect(dialog).toHaveScreenshot("catalog-dialog-open.png", {
    maxDiffPixelRatio: 0.01,
  });
});

test("UT-2.13 — catálogo: página completa", async ({ page }, testInfo) => {
  test.skip(skipIfNotMobile(testInfo));
  await openCatalog(page);
  await expect(page).toHaveScreenshot("catalog-full.png", {
    maxDiffPixelRatio: 0.01,
  });
});

test("UT-2.14 — axe: sem violações sérias no catálogo", async ({ page }, testInfo) => {
  test.skip(skipIfNotMobile(testInfo));
  const { AxeBuilder } = await import("@axe-core/playwright");
  await openCatalog(page);
  const results = await new AxeBuilder({ page }).analyze();
  const serious = results.violations.filter(
    (v) => v.impact === "critical" || v.impact === "serious",
  );
  expect(
    serious.map((v) => `${v.id}: ${v.description} (${v.nodes.length} nós)`),
  ).toEqual([]);
});
