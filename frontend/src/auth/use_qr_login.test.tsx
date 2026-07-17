import { act, renderHook, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import type { SessionResponse } from "../api/schemas";
import { useQrLogin, type QrLoginDependencies } from "./use_qr_login";

class FakeEventSource {
  onmessage: ((event: MessageEvent<string>) => void) | null = null;
  onerror: ((event: Event) => void) | null = null;
  readonly close = vi.fn();

  emit(data: unknown): void {
    this.onmessage?.(new MessageEvent("message", { data: JSON.stringify(data) }));
  }

  emitRaw(data: string): void {
    this.onmessage?.(new MessageEvent("message", { data }));
  }

  fail(): void {
    this.onerror?.(new Event("error"));
  }
}

function authenticatedSession(): SessionResponse {
  return {
    authenticated: true,
    user: { display_label: "已登录用户", identity_source: "canvas" },
    csrf_token: "memory-only-token",
    expires_at: "2030-01-01T00:00:00Z",
    download_delivery: "native_navigation",
  };
}

function dependencies(): {
  deps: QrLoginDependencies;
  source: FakeEventSource;
  claimSession: ReturnType<typeof vi.fn>;
} {
  const source = new FakeEventSource();
  const claimSession = vi.fn().mockResolvedValue(authenticatedSession());
  return {
    source,
    claimSession,
    deps: {
      startLogin: vi.fn().mockResolvedValue({
        pending_id: "opaque-pending",
        events_url: "/api/auth/qr/events/opaque-pending",
        expires_in_seconds: 300,
      }),
      claimSession,
      openEvents: vi.fn(() => source),
    },
  };
}

describe("useQrLogin", () => {
  it("claims the formal session only after authenticated SSE", async () => {
    const { deps, source, claimSession } = dependencies();
    const onAuthenticated = vi.fn();
    const { result } = renderHook(() => useQrLogin({ deps, onAuthenticated }));

    await act(async () => result.current.start());
    act(() => source.emit({ type: "qr", url: "https://qr.example.test/signed" }));
    expect(result.current.state.type).toBe("waiting");
    expect(claimSession).not.toHaveBeenCalled();

    act(() => source.emit({ type: "authenticated" }));
    await waitFor(() => expect(onAuthenticated).toHaveBeenCalledOnce());
    expect(claimSession).toHaveBeenCalledOnce();
    expect(source.close).toHaveBeenCalledOnce();
    expect(result.current.state.type).toBe("authenticated");
  });

  it("closes EventSource on cancellation and ignores late events", async () => {
    const { deps, source, claimSession } = dependencies();
    const { result, unmount } = renderHook(() => useQrLogin({ deps, onAuthenticated: vi.fn() }));

    await act(async () => result.current.start());
    act(() => result.current.cancel());
    act(() => source.emit({ type: "authenticated" }));
    expect(result.current.state.type).toBe("idle");
    expect(claimSession).not.toHaveBeenCalled();
    expect(source.close).toHaveBeenCalledOnce();

    unmount();
  });

  it("does not create two pending logins from repeated clicks", async () => {
    const { deps } = dependencies();
    const { result } = renderHook(() => useQrLogin({ deps, onAuthenticated: vi.fn() }));

    await act(async () => {
      await Promise.all([result.current.start(), result.current.start()]);
    });
    expect(deps.startLogin).toHaveBeenCalledOnce();
  });

  it("turns malformed messages and transport failures into safe retry states", async () => {
    const first = dependencies();
    const { result } = renderHook(() => useQrLogin({ deps: first.deps, onAuthenticated: vi.fn() }));
    await act(async () => result.current.start());
    act(() => first.source.emitRaw("not-json"));
    expect(result.current.state).toMatchObject({
      type: "error",
      error: { code: "INVALID_SSE_EVENT" },
    });

    await act(async () => result.current.start());
    act(() => first.source.fail());
    expect(result.current.state).toMatchObject({
      type: "error",
      error: { code: "SSE_CONNECTION_FAILED" },
    });
  });
});
