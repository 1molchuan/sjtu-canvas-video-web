import { screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import type { AuthApi } from "./api/auth";
import type { SessionResponse } from "./api/schemas";
import { AppRoutes } from "./router";
import { authenticatedSession, fakeAuthApi, renderWithProviders } from "./test/render";

describe("application routes", () => {
  it("does not flash protected content while Session status is checking", async () => {
    let resolve!: (value: SessionResponse) => void;
    const getSession = new Promise<SessionResponse>((done) => {
      resolve = done;
    });
    const authApi: AuthApi = {
      getSession: vi.fn(() => getSession),
      startLogin: vi.fn(),
      logout: vi.fn(),
    };
    renderWithProviders(<AppRoutes />, { authApi, route: "/courses" });

    expect(screen.getByText("正在检查登录状态")).toBeInTheDocument();
    expect(screen.queryByRole("heading", { name: "课程档案" })).not.toBeInTheDocument();
    resolve({ authenticated: false });
    expect(await screen.findByRole("heading", { name: "jAccount 扫码登录" })).toBeInTheDocument();
  });

  it("redirects an authenticated home request to the course archive", async () => {
    vi.spyOn(window, "fetch").mockResolvedValue(
      new Response(JSON.stringify({ courses: [] }), {
        status: 200,
        headers: { "content-type": "application/json" },
      }),
    );
    renderWithProviders(<AppRoutes />, { authApi: fakeAuthApi(authenticatedSession()), route: "/" });

    expect(await screen.findByRole("heading", { name: "课程档案" })).toBeInTheDocument();
  });

  it("keeps the privacy page public and gives unknown routes a useful exit", async () => {
    const { unmount } = renderWithProviders(<AppRoutes />, {
      authApi: fakeAuthApi({ authenticated: false }),
      route: "/privacy",
    });
    expect(await screen.findByRole("heading", { name: "隐私与使用说明" })).toBeInTheDocument();
    unmount();

    renderWithProviders(<AppRoutes />, {
      authApi: fakeAuthApi({ authenticated: false }),
      route: "/missing-page",
    });
    await waitFor(() => expect(screen.getByRole("heading", { name: "页面不存在" })).toBeInTheDocument());
    expect(screen.getByRole("link", { name: "返回首页" })).toHaveAttribute("href", "/");
  });
});
