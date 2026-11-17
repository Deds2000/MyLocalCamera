import { expect, test } from "@playwright/test";
import { e2ePorts } from "../playwright.config";

test("home do desktop", async ({ page }, testInfo) => {
  test.skip(!testInfo.project.name.startsWith("desktop"), "só roda nos projetos desktop");
  await page.goto(`http://127.0.0.1:${e2ePorts.desktop}/`);
  await expect(page).toHaveScreenshot("home.png", {
    maxDiffPixelRatio: 0.01,
    // QR contains the live pairing URI; never persist it in a snapshot.
    mask: [page.locator('[data-testid="qr-margin"] svg')],
    maskColor: "#ffffff",
  });
});

test("home do mobile", async ({ page }, testInfo) => {
  test.skip(testInfo.project.name.startsWith("desktop"), "só roda no projeto mobile");
  await page.goto(`http://127.0.0.1:${e2ePorts.mobile}/`);
  await expect(page).toHaveScreenshot("home.png", {
    maxDiffPixelRatio: 0.01,
  });
});
