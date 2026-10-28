import { expect, test } from "@playwright/test";

test("home do desktop", async ({ page }, testInfo) => {
  test.skip(
    !testInfo.project.name.startsWith("desktop"),
    "só roda nos projetos desktop",
  );
  await page.goto("http://localhost:1420/");
  await expect(page).toHaveScreenshot("home.png", {
    maxDiffPixelRatio: 0.01,
  });
});

test("home do mobile", async ({ page }, testInfo) => {
  test.skip(
    testInfo.project.name.startsWith("desktop"),
    "só roda no projeto mobile",
  );
  await page.goto("http://localhost:5173/");
  await expect(page).toHaveScreenshot("home.png", {
    maxDiffPixelRatio: 0.01,
  });
});
