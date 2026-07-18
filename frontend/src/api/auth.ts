import { ApiClient } from "./client";
import {
  qrStartResponseSchema,
  sessionResponseSchema,
  type QrStartResponse,
  type SessionResponse,
} from "./schemas";

export type AuthApi = {
  getSession: () => Promise<SessionResponse>;
  startLogin: (inviteToken?: string) => Promise<QrStartResponse>;
  logout: (csrfToken: string) => Promise<void>;
};

export function createAuthApi(client: ApiClient): AuthApi {
  return {
    getSession: () => client.get("/api/auth/session", sessionResponseSchema),
    startLogin: (inviteToken) =>
      inviteToken === undefined
        ? client.post("/api/auth/qr/start", qrStartResponseSchema)
        : client.postJson("/api/auth/qr/start", qrStartResponseSchema, {
            invite_token: inviteToken,
          }),
    logout: (csrfToken) => client.postNoContent("/api/auth/logout", csrfToken),
  };
}
