import { describe, expect, it } from "vitest";

import {
  apiErrorEnvelopeSchema,
  sessionResponseSchema,
  videoDetailResponseSchema,
} from "./schemas";

describe("API response schemas", () => {
  it("accepts both anonymous and authenticated session responses", () => {
    expect(sessionResponseSchema.parse({ authenticated: false })).toEqual({
      authenticated: false,
    });
    expect(
      sessionResponseSchema.parse({
        authenticated: true,
        user: { display_label: "已登录用户", identity_source: "canvas" },
        csrf_token: "memory-only-token",
        expires_at: "2030-01-01T00:00:00Z",
        download_delivery: "native_navigation",
      }).authenticated,
    ).toBe(true);
  });

  it("rejects a detail response that leaks an upstream URL field", () => {
    const result = videoDetailResponseSchema.safeParse({
      video: {
        id: "opaque-video",
        name: "录像",
        tracks: [
          {
            id: "opaque-track",
            kind: "unknown",
            suggested_filename: "recording.mp4",
            upstream_url: "https://forbidden.example/video",
          },
        ],
      },
    });
    expect(result.success).toBe(false);
  });

  it("parses the structured request ID without accepting extra detail", () => {
    expect(
      apiErrorEnvelopeSchema.parse({
        error: {
          code: "UPSTREAM_UNAVAILABLE",
          message: "当前无法获取这门课程的录像。",
          request_id: "request-opaque",
        },
      }),
    ).toEqual({
      error: {
        code: "UPSTREAM_UNAVAILABLE",
        message: "当前无法获取这门课程的录像。",
        request_id: "request-opaque",
      },
    });
  });
});
