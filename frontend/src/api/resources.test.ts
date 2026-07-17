import { describe, expect, it, vi } from "vitest";

import { ApiClient, PublicApiError } from "./client";
import { createCourseApi, shouldRetryQuery } from "./courses";
import { createDownloadApi, startNativeDownload } from "./downloads";

describe("resource API", () => {
  it("encodes opaque handles into fixed route templates", async () => {
    const get = vi.fn().mockResolvedValue({ videos: [] });
    const client = {
      get,
    } as unknown as ApiClient;
    const api = createCourseApi(client);

    await api.getVideos("course/with?delimiter");
    expect(get).toHaveBeenCalledWith(
      "/api/courses/course%2Fwith%3Fdelimiter/videos",
      expect.anything(),
    );
  });

  it("passes the in-memory CSRF token only to ticket POST", async () => {
    const post = vi.fn().mockResolvedValue({
      download_url: "/api/download/opaque-ticket",
      expires_in_seconds: 60,
    });
    const client = {
      post,
    } as unknown as ApiClient;
    const api = createDownloadApi(client);

    await api.issueTicket({
      courseHandle: "course",
      videoHandle: "video",
      trackHandle: "track",
      csrfToken: "memory-only-csrf",
    });
    expect(post).toHaveBeenCalledWith(
      "/api/courses/course/videos/video/tracks/track/ticket",
      expect.anything(),
      "memory-only-csrf",
    );
  });

  it("retries one transient upstream failure but never retries auth or schema errors", () => {
    const upstream = new PublicApiError({
      status: 502,
      code: "UPSTREAM_UNAVAILABLE",
      message: "上游失败",
    });
    const unauthorized = new PublicApiError({ status: 401, code: "UNAUTHORIZED", message: "过期" });
    const schema = new PublicApiError({ status: 200, code: "INVALID_RESPONSE", message: "结构错误" });

    expect(shouldRetryQuery(0, upstream)).toBe(true);
    expect(shouldRetryQuery(1, upstream)).toBe(false);
    expect(shouldRetryQuery(0, unauthorized)).toBe(false);
    expect(shouldRetryQuery(0, schema)).toBe(false);
  });
});

describe("native download", () => {
  it("clicks a temporary anchor without fetching or creating a Blob", () => {
    const fetchSpy = vi.spyOn(window, "fetch");
    const blobSpy = vi.spyOn(URL, "createObjectURL");
    const clickSpy = vi.spyOn(HTMLAnchorElement.prototype, "click").mockImplementation(() => undefined);

    startNativeDownload("/api/download/opaque-ticket");

    expect(clickSpy).toHaveBeenCalledOnce();
    expect(fetchSpy).not.toHaveBeenCalled();
    expect(blobSpy).not.toHaveBeenCalled();
    expect(document.querySelector('a[href*="opaque-ticket"]')).not.toBeInTheDocument();
  });

  it("rejects a URL outside the fixed download route", () => {
    expect(() => startNativeDownload("https://upstream.example/video")).toThrow("download path");
  });
});
