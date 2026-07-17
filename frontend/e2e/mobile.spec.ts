import { expect, test } from "@playwright/test";

test("360px-class viewport has no horizontal overflow and remains keyboard operable", async ({ page }) => {
  await page.goto("/login");
  const viewport = page.viewportSize();
  expect(viewport?.width).toBeLessThanOrEqual(400);
  const overflow = await page.evaluate(() => document.documentElement.scrollWidth - document.documentElement.clientWidth);
  expect(overflow).toBeLessThanOrEqual(1);

  await page.keyboard.press("Tab");
  await expect(page.locator(":focus-visible")).toBeVisible();
  await expect(page.getByRole("button", { name: "开始扫码登录" })).toBeVisible();
});
