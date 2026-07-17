import { ApiClient } from "./client";
import {
  qrStartResponseSchema,
  sessionResponseSchema,
  type QrStartResponse,
  type SessionResponse,
} from "./schemas";

export type AuthApi = {
  getSession: () => Promise<SessionResponse>;
  startLogin: () => Promise<QrStartResponse>;
  logout: (csrfToken: string) => Promise<void>;
};

export function createAuthApi(client: ApiClient): AuthApi {
  return {
    getSession: () => client.get("/api/auth/session", sessionResponseSchema),
    startLogin: () => client.post("/api/auth/qr/start", qrStartResponseSchema),
    logout: (csrfToken) => client.postNoContent("/api/auth/logout", csrfToken),
  };
}
