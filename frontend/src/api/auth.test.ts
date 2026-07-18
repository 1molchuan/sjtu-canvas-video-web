import { describe, expect, it, vi } from "vitest";

import { createAuthApi } from "./auth";
import { ApiClient } from "./client";

describe("AuthApi invitations", () => {
  it("sends the invite only in the QR start JSON body", async () => {
    const fetcher = vi.fn().mockResolvedValue(
      new Response(
        JSON.stringify({
          pending_id: "pending",
          events_url: "/api/auth/qr/events/pending",
          expires_in_seconds: 300,
        }),
        { status: 200, headers: { "content-type": "application/json" } },
      ),
    );
    const api = createAuthApi(new ApiClient({ fetcher }));

    await api.startLogin("invite-secret");

    expect(fetcher).toHaveBeenCalledOnce();
    const [path, init] = fetcher.mock.calls[0] as [string, RequestInit];
    expect(path).toBe("/api/auth/qr/start");
    expect(path).not.toContain("invite-secret");
    expect(typeof init.body).toBe("string");
    if (typeof init.body !== "string") throw new Error("expected JSON request body");
    expect(JSON.parse(init.body) as unknown).toEqual({ invite_token: "invite-secret" });
  });
});
