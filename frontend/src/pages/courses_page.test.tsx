import { screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

import type { CourseApi } from "../api/courses";
import { renderWithProviders } from "../test/render";
import { CoursesPage } from "./courses_page";

describe("CoursesPage", () => {
  it("renders course metadata without exposing its opaque handle", async () => {
    const api: CourseApi = {
      getCourses: vi.fn().mockResolvedValue({
        courses: [
          { id: "opaque-course-handle", name: "软件工程", course_code: "SE101", term_name: null },
        ],
      }),
      getVideos: vi.fn(),
      getVideoDetail: vi.fn(),
    };
    renderWithProviders(<CoursesPage api={api} />, { route: "/courses" });

    expect(await screen.findByText("软件工程")).toBeInTheDocument();
    expect(screen.getByText("SE101")).toBeInTheDocument();
    expect(screen.queryByText("opaque-course-handle")).not.toBeInTheDocument();
  });

  it("renders a real empty state and supports explicit refresh", async () => {
    const user = userEvent.setup();
    const getCourses = vi.fn().mockResolvedValue({ courses: [] });
    const api: CourseApi = { getCourses, getVideos: vi.fn(), getVideoDetail: vi.fn() };
    renderWithProviders(<CoursesPage api={api} />, { route: "/courses" });

    expect(await screen.findByText("暂时没有可访问的课程")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "刷新课程" }));
    await waitFor(() => expect(getCourses).toHaveBeenCalledTimes(2));
  });
});
