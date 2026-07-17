import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { act, renderHook, waitFor } from "@testing-library/react";
import type { PropsWithChildren } from "react";
import { describe, expect, it, vi } from "vitest";

import { PublicApiError } from "../api/client";
import type { AuthApi } from "../api/auth";
import type { SessionResponse } from "../api/schemas";
import { useAuth } from "./auth_context";
import { AuthProvider } from "./auth_provider";

function session(): Extract<SessionResponse, { authenticated: true }> {
  return {
    authenticated: true,
    user: { display_label: "已登录用户", identity_source: "my_sjtu" },
    csrf_token: "memory-only-csrf",
    expires_at: "2030-01-01T00:00:00Z",
  };
}

function setup(getSession: AuthApi["getSession"]) {
  const queryClient = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  const api: AuthApi = {
    getSession: vi.fn(getSession),
    startLogin: vi.fn(),
    logout: vi.fn().mockResolvedValue(undefined),
  };
  const wrapper = ({ children }: PropsWithChildren) => (
    <QueryClientProvider client={queryClient}>
      <AuthProvider api={api}>{children}</AuthProvider>
    </QueryClientProvider>
  );
  return { api, queryClient, wrapper };
}

describe("AuthProvider", () => {
  it("checks the server before classifying the browser as anonymous", async () => {
    let resolve!: (value: SessionResponse) => void;
    const pending = new Promise<SessionResponse>((done) => {
      resolve = done;
    });
    const { wrapper } = setup(() => pending);
    const { result } = renderHook(() => useAuth(), { wrapper });
    expect(result.current.state.status).toBe("checking");

    act(() => resolve({ authenticated: false }));
    await waitFor(() => expect(result.current.state.status).toBe("anonymous"));
  });

  it("restores an authenticated session without persistent client storage", async () => {
    const { wrapper } = setup(() => Promise.resolve(session()));
    const { result } = renderHook(() => useAuth(), { wrapper });

    await waitFor(() => expect(result.current.state.status).toBe("authenticated"));
    expect(result.current.session?.csrf_token).toBe("memory-only-csrf");
    expect(window.localStorage.length).toBe(0);
    expect(window.sessionStorage.length).toBe(0);
  });

  it("uses the in-memory CSRF token and clears cached private data on logout", async () => {
    const { wrapper, api, queryClient } = setup(() => Promise.resolve(session()));
    queryClient.setQueryData(["courses"], { private: true });
    const { result } = renderHook(() => useAuth(), { wrapper });
    await waitFor(() => expect(result.current.state.status).toBe("authenticated"));

    await act(async () => result.current.logout());
    expect(api.logout).toHaveBeenCalledWith("memory-only-csrf");
    expect(queryClient.getQueryData(["courses"])).toBeUndefined();
    expect(result.current.state.status).toBe("anonymous");
  });

  it("classifies an expired Session response without retaining CSRF", async () => {
    const expired = new PublicApiError({
      status: 401,
      code: "SESSION_EXPIRED",
      message: "登录状态已过期。",
    });
    const { wrapper } = setup(async () => Promise.reject(expired));
    const { result } = renderHook(() => useAuth(), { wrapper });

    await waitFor(() => expect(result.current.state.status).toBe("expired"));
    expect(result.current.session).toBeNull();
  });
});
