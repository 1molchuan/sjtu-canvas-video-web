import { screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { Route, Routes } from "react-router-dom";
import { describe, expect, it, vi } from "vitest";

import { PublicApiError } from "../api/client";
import type { CourseApi } from "../api/courses";
import { renderWithProviders } from "../test/render";
import { CourseVideosPage } from "./course_videos_page";

function renderPage(api: CourseApi) {
  return renderWithProviders(
    <Routes>
      <Route path="/courses/:courseHandle" element={<CourseVideosPage api={api} />} />
    </Routes>,
    { route: "/courses/opaque-course" },
  );
}

describe("CourseVideosPage", () => {
  it("loads only the selected course and renders its recordings", async () => {
    const api: CourseApi = {
      getCourses: vi.fn(),
      getVideos: vi.fn().mockResolvedValue({
        videos: [{ id: "opaque-video", name: "第 1 讲", started_at: "2030-01-01T09:00:00Z" }],
      }),
      getVideoDetail: vi.fn(),
    };
    renderPage(api);

    expect(await screen.findByText("第 1 讲")).toBeInTheDocument();
    expect(api.getVideos).toHaveBeenCalledWith("opaque-course");
    expect(api.getCourses).not.toHaveBeenCalled();
  });

  it("keeps a 502 visible with request ID and never turns it into an empty list", async () => {
    const error = new PublicApiError({
      status: 502,
      code: "UPSTREAM_UNAVAILABLE",
      message: "上游服务异常。",
      requestId: "request-safe-502",
    });
    const getVideos = vi.fn().mockRejectedValue(error);
    const api: CourseApi = { getCourses: vi.fn(), getVideos, getVideoDetail: vi.fn() };
    renderPage(api);

    expect(await screen.findByText("当前无法获取这门课程的录像。")).toBeInTheDocument();
    expect(screen.getByText(/request-safe-502/)).toBeInTheDocument();
    expect(screen.queryByText("这门课程暂无录像")).not.toBeInTheDocument();
    expect(getVideos).toHaveBeenCalledTimes(2);
  });

  it("offers one explicit retry after a failed automatic retry", async () => {
    const user = userEvent.setup();
    const getVideos = vi
      .fn()
      .mockRejectedValueOnce(new PublicApiError({ status: 502, code: "UPSTREAM_UNAVAILABLE", message: "失败" }))
      .mockRejectedValueOnce(new PublicApiError({ status: 502, code: "UPSTREAM_UNAVAILABLE", message: "失败" }))
      .mockResolvedValue({ videos: [] });
    const api: CourseApi = { getCourses: vi.fn(), getVideos, getVideoDetail: vi.fn() };
    renderPage(api);

    await user.click(await screen.findByRole("button", { name: "重试" }));
    expect(await screen.findByText("这门课程暂无录像")).toBeInTheDocument();
    await waitFor(() => expect(getVideos).toHaveBeenCalledTimes(3));
  });
});
