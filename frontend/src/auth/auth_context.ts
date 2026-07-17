import { createContext, useContext } from "react";

import type { AuthApi } from "../api/auth";
import type { ApiClient, PublicApiError } from "../api/client";
import type { SessionResponse } from "../api/schemas";

export type AuthenticatedSession = Extract<SessionResponse, { authenticated: true }>;

export type AuthState =
  | { status: "checking" }
  | { status: "authenticated"; session: AuthenticatedSession }
  | { status: "anonymous" }
  | { status: "expired" }
  | { status: "error"; error: PublicApiError };

export type AuthContextValue = {
  state: AuthState;
  session: AuthenticatedSession | null;
  apiClient: ApiClient;
  authApi: AuthApi;
  refresh: () => Promise<SessionResponse>;
  logout: () => Promise<void>;
};

export const AuthContext = createContext<AuthContextValue | null>(null);

export function useAuth(): AuthContextValue {
  const value = useContext(AuthContext);
  if (value === null) {
    throw new Error("useAuth must be used inside AuthProvider");
  }
  return value;
}
