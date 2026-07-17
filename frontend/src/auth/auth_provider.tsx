import { useQueryClient } from "@tanstack/react-query";
import {
  useCallback,
  useEffect,
  useMemo,
  useState,
  type PropsWithChildren,
} from "react";

import { createAuthApi, type AuthApi } from "../api/auth";
import { ApiClient, PublicApiError } from "../api/client";
import type { SessionResponse } from "../api/schemas";
import { AuthContext, type AuthState } from "./auth_context";

type AuthProviderProps = PropsWithChildren<{ api?: AuthApi }>;

export function AuthProvider({ children, api }: AuthProviderProps) {
  const queryClient = useQueryClient();
  const [state, setState] = useState<AuthState>({ status: "checking" });
  const expire = useCallback(() => {
    queryClient.clear();
    setState({ status: "expired" });
  }, [queryClient]);
  const apiClient = useMemo(() => new ApiClient({ onUnauthorized: expire }), [expire]);
  const authApi = useMemo(() => api ?? createAuthApi(apiClient), [api, apiClient]);

  const refresh = useCallback(async () => {
    try {
      const response = await authApi.getSession();
      setState(response.authenticated ? { status: "authenticated", session: response } : { status: "anonymous" });
      return response;
    } catch (error) {
      const publicError = asPublicError(error);
      setState(classifySessionError(publicError));
      throw publicError;
    }
  }, [authApi]);

  const logout = useCallback(async () => {
    if (state.status === "authenticated") {
      await authApi.logout(state.session.csrf_token);
    }
    queryClient.clear();
    setState({ status: "anonymous" });
  }, [authApi, queryClient, state]);

  useEffect(() => {
    let active = true;
    authApi.getSession().then(
      (response) => {
        if (active) setState(stateForSession(response));
      },
      (error: unknown) => {
        if (active) setState(classifySessionError(asPublicError(error)));
      },
    );
    return () => {
      active = false;
    };
  }, [authApi]);

  useEffect(() => {
    const onFocus = () => {
      if (state.status === "authenticated") {
        void refresh().catch(() => undefined);
      }
    };
    window.addEventListener("focus", onFocus);
    return () => window.removeEventListener("focus", onFocus);
  }, [refresh, state.status]);

  const session = state.status === "authenticated" ? state.session : null;
  const value = useMemo(
    () => ({ state, session, apiClient, authApi, refresh, logout }),
    [apiClient, authApi, logout, refresh, session, state],
  );
  return <AuthContext.Provider value={value}>{children}</AuthContext.Provider>;
}

function stateForSession(response: SessionResponse): AuthState {
  return response.authenticated
    ? { status: "authenticated", session: response }
    : { status: "anonymous" };
}

function classifySessionError(error: PublicApiError): AuthState {
  if (error.status === 401 || error.code === "SESSION_EXPIRED") {
    return { status: "expired" };
  }
  return { status: "error", error };
}

function asPublicError(error: unknown): PublicApiError {
  if (error instanceof PublicApiError) return error;
  return new PublicApiError({
    status: 0,
    code: "SESSION_CHECK_FAILED",
    message: "无法检查登录状态，请重试。",
  });
}
