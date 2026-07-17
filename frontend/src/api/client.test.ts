import { z } from "zod";
import { describe, expect, it, vi } from "vitest";

import { ApiClient, PublicApiError } from "./client";

const okSchema = z.object({ ok: z.literal(true) }).strict();

describe("ApiClient", () => {
  it("uses same-origin credentials and validates JSON", async () => {
    const fetcher = vi.fn<typeof fetch>().mockResolvedValue(
      new Response(JSON.stringify({ ok: true }), {
        status: 200,
        headers: { "content-type": "application/json" },
      }),
    );
    const client = new ApiClient({ fetcher });

    await expect(client.get("/api/example", okSchema)).resolves.toEqual({ ok: true });
    expect(fetcher).toHaveBeenCalledWith(
      "/api/example",
      expect.objectContaining({ credentials: "same-origin", method: "GET" }),
    );
  });

  it("surfaces a 502 request ID instead of converting it to empty data", async () => {
    const fetcher = vi.fn<typeof fetch>().mockResolvedValue(
      new Response(
        JSON.stringify({
          error: {
            code: "UPSTREAM_UNAVAILABLE",
            message: "上游服务暂时不可用。",
            request_id: "request-502",
          },
        }),
        { status: 502, headers: { "content-type": "application/json" } },
      ),
    );
    const client = new ApiClient({ fetcher });

    await expect(client.get("/api/courses/opaque/videos", okSchema)).rejects.toMatchObject({
      name: "PublicApiError",
      status: 502,
      code: "UPSTREAM_UNAVAILABLE",
      requestId: "request-502",
    });
  });

  it("notifies the auth boundary when a request returns 401", async () => {
    const onUnauthorized = vi.fn();
    const fetcher = vi.fn<typeof fetch>().mockResolvedValue(
      new Response("", { status: 401, headers: { "content-type": "text/plain" } }),
    );
    const client = new ApiClient({ fetcher, onUnauthorized });

    await expect(client.get("/api/courses", okSchema)).rejects.toBeInstanceOf(PublicApiError);
    expect(onUnauthorized).toHaveBeenCalledOnce();
  });

  it("rejects successful non-JSON responses without serializing the body", async () => {
    const fetcher = vi.fn<typeof fetch>().mockResolvedValue(
      new Response("private response", {
        status: 200,
        headers: { "content-type": "text/html" },
      }),
    );
    const client = new ApiClient({ fetcher });

    await expect(client.get("/api/example", okSchema)).rejects.toMatchObject({
      code: "INVALID_RESPONSE",
    });
  });
});
