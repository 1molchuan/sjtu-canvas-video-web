import { screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { Route, Routes } from "react-router-dom";
import { describe, expect, it, vi } from "vitest";

import type { CourseApi } from "../api/courses";
import type { DirectDownloadAdapter, DirectDownloadFile } from "../api/direct_download";
import type { DownloadApi } from "../api/downloads";
import type { AuthApi } from "../api/auth";
import { authenticatedSession, fakeAuthApi, renderWithProviders } from "../test/render";
import { VideoPage } from "./video_page";

type RenderOptions = {
  api: CourseApi;
  downloads?: DownloadApi;
  startDownload?: (url: string) => void;
  directDownload?: DirectDownloadAdapter;
  authApi?: AuthApi;
};

function renderPage(options: RenderOptions) {
  return renderWithProviders(
    <Routes>
      <Route
        path="/courses/:courseHandle/videos/:videoHandle"
        element={
          <VideoPage
            api={options.api}
            downloads={options.downloads}
            startDownload={options.startDownload}
            directDownload={options.directDownload}
          />
        }
      />
    </Routes>,
    { authApi: options.authApi, route: "/courses/opaque-course/videos/opaque-video" },
  );
}

function detailApi(): CourseApi {
  return {
    getCourses: vi.fn(),
    getVideos: vi.fn(),
    getVideoDetail: vi.fn().mockResolvedValue({
      video: {
        id: "opaque-video",
        name: "课程录像",
        tracks: [
          { id: "track-one", kind: "unknown", suggested_filename: "track-one.mp4" },
          { id: "track-two", kind: "unknown", suggested_filename: "track-two.mp4" },
        ],
      },
    }),
  };
}

describe("VideoPage", () => {
  it("uses neutral numbered labels for unknown tracks", async () => {
    renderPage({ api: detailApi() });

    expect(await screen.findByText("视频轨道 1")).toBeInTheDocument();
    expect(screen.getByText("视频轨道 2")).toBeInTheDocument();
    expect(screen.getAllByText("类型未识别")).toHaveLength(2);
    expect(screen.queryByText("电脑录屏")).not.toBeInTheDocument();
  });

  it("issues a fresh CSRF-bound ticket then starts native navigation", async () => {
    const user = userEvent.setup();
    const issueTicket = vi.fn().mockResolvedValue({
      download_url: "/api/download/short-lived-ticket",
      expires_in_seconds: 60,
    });
    const startDownload = vi.fn();
    renderPage({ api: detailApi(), downloads: { issueTicket }, startDownload });

    await user.click(await screen.findByRole("button", { name: "下载视频轨道 1" }));
    await waitFor(() => expect(startDownload).toHaveBeenCalledWith("/api/download/short-lived-ticket"));
    expect(issueTicket).toHaveBeenCalledWith({
      courseHandle: "opaque-course",
      videoHandle: "opaque-video",
      trackHandle: "track-one",
      csrfToken: "memory-only-csrf",
    });
    expect(screen.queryByText("short-lived-ticket")).not.toBeInTheDocument();
  });

  it("selects a destination before issuing a ticket and streams directly", async () => {
    const user = userEvent.setup();
    const file = { createWritable: vi.fn() } as unknown as DirectDownloadFile;
    const selectFile = vi.fn().mockResolvedValue(file);
    const stream = vi.fn().mockResolvedValue(undefined);
    const issueTicket = vi.fn().mockResolvedValue({
      download_url: "/api/download/direct-ticket",
      expires_in_seconds: 60,
    });
    const startDownload = vi.fn();
    const session = { ...authenticatedSession(), download_delivery: "direct_stream" as const };
    renderPage({
      api: detailApi(),
      downloads: { issueTicket },
      startDownload,
      directDownload: { selectFile, stream },
      authApi: fakeAuthApi(session),
    });

    await user.click(await screen.findByRole("button", { name: "下载视频轨道 1" }));
    await waitFor(() => expect(stream).toHaveBeenCalledWith("/api/download/direct-ticket", file));
    expect(selectFile).toHaveBeenCalledWith("track-one.mp4");
    expect(selectFile.mock.invocationCallOrder[0]).toBeLessThan(issueTicket.mock.invocationCallOrder[0]);
    expect(startDownload).not.toHaveBeenCalled();
    expect(screen.getByText("直连下载已完成。")).toBeInTheDocument();
  });
});
