import { expect, type Page } from "@playwright/test";

export async function login(page: Page, verifyProgress = false): Promise<void> {
  await page.goto("/");
  await expect(page).toHaveURL(/\/login$/);
  await page.getByRole("button", { name: "开始扫码登录" }).click();
  if (verifyProgress) {
    await expect(page.getByRole("img", { name: "jAccount 登录二维码" })).toBeVisible();
    await expect(page.getByText("已扫码，等待确认")).toBeVisible();
    await expect(page.getByText("正在建立 Canvas 会话")).toBeVisible();
  }
  await expect(page).toHaveURL(/\/courses$/);
  await expect(page.getByRole("heading", { name: "课程档案" })).toBeVisible();
}

export async function openSuccessfulCourse(page: Page): Promise<void> {
  const card = page.locator("article").filter({
    has: page.getByRole("heading", { name: "可用课程", exact: true }),
  });
  await card.getByRole("link", { name: /查看课程录像/ }).click();
  await expect(page.getByRole("heading", { name: "可用课程" })).toBeVisible();
  await expect(page.getByText("第一讲：课程介绍")).toBeVisible();
}

export async function openVideo(page: Page): Promise<void> {
  await openSuccessfulCourse(page);
  await page.getByRole("link", { name: "查看轨道" }).click();
  await expect(page.getByRole("heading", { name: "第一讲：课程介绍" })).toBeVisible();
}
