import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render } from "@testing-library/react";
import { MemoryRouter } from "react-router-dom";
import type { ReactElement } from "react";
import { vi } from "vitest";

import type { AuthApi } from "../api/auth";
import type { SessionResponse } from "../api/schemas";
import { AuthProvider } from "../auth/auth_provider";

export function authenticatedSession(): Extract<SessionResponse, { authenticated: true }> {
  return {
    authenticated: true,
    user: { display_label: "已登录用户", identity_source: "canvas" },
    csrf_token: "memory-only-csrf",
    expires_at: "2030-01-01T00:00:00Z",
  };
}

export function fakeAuthApi(session: SessionResponse = authenticatedSession()): AuthApi {
  return {
    getSession: vi.fn().mockResolvedValue(session),
    startLogin: vi.fn(),
    logout: vi.fn().mockResolvedValue(undefined),
  };
}

export function renderWithProviders(
  element: ReactElement,
  options: { authApi?: AuthApi; route?: string } = {},
) {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false, gcTime: 0 } },
  });
  const authApi = options.authApi ?? fakeAuthApi();
  const result = render(
    <QueryClientProvider client={queryClient}>
      <AuthProvider api={authApi}>
        <MemoryRouter initialEntries={[options.route ?? "/"]}>{element}</MemoryRouter>
      </AuthProvider>
    </QueryClientProvider>,
  );
  return { ...result, queryClient, authApi };
}
