import { PublicApiError, type ApiClient } from "./client";
import {
  coursesResponseSchema,
  videoDetailResponseSchema,
  videosResponseSchema,
  type CoursesResponse,
  type VideoDetailResponse,
  type VideosResponse,
} from "./schemas";

export type CourseApi = {
  getCourses: () => Promise<CoursesResponse>;
  getVideos: (courseHandle: string) => Promise<VideosResponse>;
  getVideoDetail: (courseHandle: string, videoHandle: string) => Promise<VideoDetailResponse>;
};

export function createCourseApi(client: ApiClient): CourseApi {
  return {
    getCourses: () => client.get("/api/courses", coursesResponseSchema),
    getVideos: (courseHandle) =>
      client.get(`/api/courses/${encodeHandle(courseHandle)}/videos`, videosResponseSchema),
    getVideoDetail: (courseHandle, videoHandle) =>
      client.get(
        `/api/courses/${encodeHandle(courseHandle)}/videos/${encodeHandle(videoHandle)}`,
        videoDetailResponseSchema,
      ),
  };
}

export function shouldRetryQuery(failureCount: number, error: Error): boolean {
  if (failureCount >= 1 || !(error instanceof PublicApiError)) return false;
  return error.status === 0 || error.status === 502 || error.status === 504;
}

export function encodeHandle(handle: string): string {
  return encodeURIComponent(handle);
}
