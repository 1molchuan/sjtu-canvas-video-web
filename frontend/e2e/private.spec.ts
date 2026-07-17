import { expect, test } from "@playwright/test";

import { login, openSuccessfulCourse, openVideo } from "./helpers";

test.beforeEach(async ({ page }) => {
  await login(page);
});

test("course upstream 502 remains an error with a safe request ID", async ({ page }) => {
  const card = page.locator("article").filter({
    has: page.getByRole("heading", { name: "暂不可用课程", exact: true }),
  });
  await card.getByRole("link", { name: /查看课程录像/ }).click();
  await expect(page.getByText("当前无法获取这门课程的录像。")).toBeVisible();
  await expect(page.getByText(/fixture-request-502/)).toBeVisible();
  await expect(page.getByText("这门课程暂无录像")).toHaveCount(0);
  await page.getByRole("button", { name: "重试" }).click();
  await expect(page.getByText("当前无法获取这门课程的录像。")).toBeVisible();
});

test("successful course loads only after navigation", async ({ page }) => {
  await openSuccessfulCourse(page);
  await expect(page.getByText("共 1 条录像")).toBeVisible();
  await expect(page.getByText("opaque-course-success")).toHaveCount(0);
});

test("two unknown tracks stay neutral and disclose no capability values", async ({ page }) => {
  await openVideo(page);
  await expect(page.getByRole("heading", { name: "视频轨道 1" })).toBeVisible();
  await expect(page.getByRole("heading", { name: "视频轨道 2" })).toBeVisible();
  await expect(page.getByText("类型未识别")).toHaveCount(2);
  await expect(page.getByText("电脑录屏")).toHaveCount(0);
  await expect(page.getByText(/fixture-ticket|opaque-track|upstream/i)).toHaveCount(0);
});

test("ticket starts a native browser download", async ({ page }) => {
  await openVideo(page);
  const downloadPromise = page.waitForEvent("download");
  await page.getByRole("button", { name: "下载视频轨道 1" }).click();
  const download = await downloadPromise;
  expect(download.suggestedFilename()).toBe("lecture-track-1.mp4");
  await expect(page.getByText(/下载已开始/)).toBeVisible();
});

test("expired session returns to login without private data", async ({ page }) => {
  await page.request.post("/__fixture/expire");
  await page.reload();
  await expect(page).toHaveURL(/\/login$/);
  await expect(page.getByRole("heading", { name: "课程档案" })).toHaveCount(0);
});

test("logout clears Session and invalidates an old ticket", async ({ page }) => {
  await openVideo(page);
  const responsePromise = page.waitForResponse((response) =>
    response.url().includes("/tracks/") && response.url().endsWith("/ticket"),
  );
  const downloadPromise = page.waitForEvent("download");
  await page.getByRole("button", { name: "下载视频轨道 1" }).click();
  const ticketResponse = await responsePromise;
  await downloadPromise;
  const ticket = (await ticketResponse.json()) as { download_url: string };
  await page.goto("/courses");
  await page.getByRole("button", { name: "登出" }).click();
  await expect(page).toHaveURL(/\/login$/);
  const oldTicket = await page.request.get(ticket.download_url);
  expect(oldTicket.status()).toBe(401);
});
