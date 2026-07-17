import { act, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

import type { LoginEventStream, QrLoginDependencies } from "../auth/use_qr_login";
import { fakeAuthApi, renderWithProviders } from "../test/render";
import { LoginPage } from "./login_page";

class FakeSource implements LoginEventStream {
  onmessage: ((event: MessageEvent<string>) => void) | null = null;
  onerror: ((event: Event) => void) | null = null;
  close = vi.fn();

  emit(event: unknown): void {
    this.onmessage?.(new MessageEvent("message", { data: JSON.stringify(event) }));
  }
}

function loginDependencies(source: FakeSource): QrLoginDependencies {
  return {
    startLogin: vi.fn().mockResolvedValue({
      pending_id: "opaque-pending",
      events_url: "/api/auth/qr/events/opaque-pending",
      expires_in_seconds: 300,
    }),
    claimSession: vi.fn().mockResolvedValue({
      authenticated: true,
      user: { display_label: "已登录用户", identity_source: "canvas" },
      csrf_token: "memory-only-csrf",
      expires_at: "2030-01-01T00:00:00Z",
    }),
    openEvents: vi.fn(() => source),
  };
}

describe("LoginPage", () => {
  it("renders the QR locally and maps scan progress without showing its URL", async () => {
    const user = userEvent.setup();
    const source = new FakeSource();
    renderWithProviders(<LoginPage dependencies={loginDependencies(source)} />, {
      authApi: fakeAuthApi({ authenticated: false }),
      route: "/login",
    });
    await waitFor(() => expect(screen.getByRole("button", { name: "开始扫码登录" })).toBeEnabled());
    await user.click(screen.getByRole("button", { name: "开始扫码登录" }));

    act(() => source.emit({ type: "qr", url: "https://qr.example.test/signed-secret" }));
    expect(screen.getByRole("img", { name: "jAccount 登录二维码" })).toBeInTheDocument();
    expect(screen.queryByText("https://qr.example.test/signed-secret")).not.toBeInTheDocument();
    act(() => source.emit({ type: "scanned" }));
    expect(screen.getByText("已扫码，等待确认")).toBeInTheDocument();
    act(() => source.emit({ type: "authenticating" }));
    expect(screen.getByText("正在建立 Canvas 会话")).toBeInTheDocument();
  });

  it("shows invitation rejection and QR expiry as different terminal states", async () => {
    const user = userEvent.setup();
    const source = new FakeSource();
    renderWithProviders(<LoginPage dependencies={loginDependencies(source)} />, {
      authApi: fakeAuthApi({ authenticated: false }),
      route: "/login",
    });
    await user.click(await screen.findByRole("button", { name: "开始扫码登录" }));
    act(() => source.emit({ type: "rejected" }));
    expect(screen.getByRole("alert")).toHaveTextContent("不在邀请名单");
  });
});
