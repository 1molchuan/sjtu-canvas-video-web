import { expect, test } from "@playwright/test";

import { login } from "./helpers";

test("anonymous home redirects without flashing private course content", async ({ page }) => {
  await page.goto("/");
  await expect(page).toHaveURL(/\/login$/);
  await expect(page.getByRole("heading", { name: "jAccount 扫码登录" })).toBeVisible();
  await expect(page.getByText("非上海交通大学官方服务", { exact: true })).toBeVisible();
  await expect(page.getByRole("heading", { name: "课程档案" })).toHaveCount(0);
});

test("QR and SSE complete the formal browser session", async ({ page }) => {
  await login(page, true);
  await page.reload();
  await expect(page.getByRole("heading", { name: "课程档案" })).toBeVisible();
  await expect(page.getByText("https://qr.example.test/fixture")).toHaveCount(0);
});

test("privacy statement is public and contains the storage boundary", async ({ page }) => {
  await page.goto("/privacy");
  await expect(page.getByRole("heading", { name: "隐私与使用说明" })).toBeVisible();
  await expect(page.getByText(/课程视频不落盘/)).toBeVisible();
  await expect(page.getByText(/不会把上游 Cookie/)).toBeVisible();
});
