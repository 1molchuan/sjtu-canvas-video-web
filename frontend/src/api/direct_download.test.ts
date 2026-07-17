import { afterEach, describe, expect, it, vi } from "vitest";

import {
  DirectDownloadUnsupportedError,
  selectDirectDownloadFile,
  streamDirectDownload,
  type DirectDownloadFile,
} from "./direct_download";

afterEach(() => {
  Reflect.deleteProperty(window, "showSaveFilePicker");
});

describe("direct download", () => {
  it("selects an MP4 destination before network delivery", async () => {
    const file = fakeFile();
    const picker = vi.fn().mockResolvedValue(file);
    Object.defineProperty(window, "showSaveFilePicker", { configurable: true, value: picker });

    await expect(selectDirectDownloadFile("lecture.mp4")).resolves.toBe(file);
    expect(picker).toHaveBeenCalledWith({
      suggestedName: "lecture.mp4",
      types: [{ description: "MP4 视频", accept: { "video/mp4": [".mp4"] } }],
    });
  });

  it("treats picker cancellation as an intentional no-op", async () => {
    const picker = vi.fn().mockRejectedValue(new DOMException("cancelled", "AbortError"));
    Object.defineProperty(window, "showSaveFilePicker", { configurable: true, value: picker });

    await expect(selectDirectDownloadFile("lecture.mp4")).resolves.toBeNull();
  });

  it("reports unsupported browsers explicitly", async () => {
    await expect(selectDirectDownloadFile("lecture.mp4")).rejects.toBeInstanceOf(
      DirectDownloadUnsupportedError,
    );
  });

  it("streams the response body into the selected file without a Blob", async () => {
    const bytes: number[] = [];
    const file = fakeFile(bytes);
    const fetchSpy = vi.spyOn(window, "fetch").mockResolvedValue(
      new Response(Uint8Array.of(1, 2, 3), { status: 206 }),
    );
    const blobSpy = vi.spyOn(URL, "createObjectURL");

    await streamDirectDownload("/api/download/opaque-ticket", file);

    expect(fetchSpy).toHaveBeenCalledWith("/api/download/opaque-ticket", {
      credentials: "same-origin",
      cache: "no-store",
      redirect: "follow",
    });
    expect(bytes).toEqual([1, 2, 3]);
    expect(blobSpy).not.toHaveBeenCalled();
  });

  it("rejects destinations outside the same-origin ticket route", async () => {
    await expect(streamDirectDownload("https://upstream.example/video", fakeFile())).rejects.toThrow(
      "download path",
    );
  });
});

function fakeFile(bytes: number[] = []): DirectDownloadFile {
  return {
    createWritable: vi.fn().mockResolvedValue(
      new WritableStream<Uint8Array>({
        write(chunk) {
          bytes.push(...chunk);
        },
      }),
    ),
  };
}
