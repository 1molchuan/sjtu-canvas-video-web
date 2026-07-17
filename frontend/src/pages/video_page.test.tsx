import { screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { Route, Routes } from "react-router-dom";
import { describe, expect, it, vi } from "vitest";

import type { CourseApi } from "../api/courses";
import type { DownloadApi } from "../api/downloads";
import { renderWithProviders } from "../test/render";
import { VideoPage } from "./video_page";

function renderPage(options: { api: CourseApi; downloads?: DownloadApi; startDownload?: (url: string) => void }) {
  return renderWithProviders(
    <Routes>
      <Route
        path="/courses/:courseHandle/videos/:videoHandle"
        element={<VideoPage api={options.api} downloads={options.downloads} startDownload={options.startDownload} />}
      />
    </Routes>,
    { route: "/courses/opaque-course/videos/opaque-video" },
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
});
