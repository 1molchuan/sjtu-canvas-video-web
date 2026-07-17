import { useQuery } from "@tanstack/react-query";
import { useMemo } from "react";
import { Link, useLocation, useParams } from "react-router-dom";

import { createCourseApi, shouldRetryQuery, type CourseApi } from "../api/courses";
import { useAuth } from "../auth/auth_context";
import { AppShell } from "../components/app_shell";
import { EmptyState } from "../components/empty_state";
import { ErrorNotice } from "../components/error_notice";
import { LoadingState } from "../components/loading_state";
import { VideoCard } from "../components/video_card";

export function CourseVideosPage({ api }: { api?: CourseApi }) {
  const { courseHandle = "" } = useParams();
  const location = useLocation();
  const auth = useAuth();
  const courseApi = useMemo(() => api ?? createCourseApi(auth.apiClient), [api, auth.apiClient]);
  const query = useQuery({
    queryKey: ["videos", courseHandle],
    queryFn: () => courseApi.getVideos(courseHandle),
    enabled: auth.session !== null && courseHandle.length > 0,
    retry: shouldRetryQuery,
    retryDelay: 0,
  });
  const courseName = navigationText(location.state, "courseName") ?? "课程录像";

  return (
    <AppShell actions={<Link className="button button--quiet" to="/courses">返回课程列表</Link>}>
      <header className="page-heading page-heading--with-rule">
        <p className="eyebrow">课程录像</p>
        <h1>{courseName}</h1>
        {query.data !== undefined && <p>共 {query.data.videos.length} 条录像</p>}
      </header>
      {query.isPending && <LoadingState label="正在获取课程录像" />}
      {query.error !== null && (
        <ErrorNotice
          error={query.error}
          title="当前无法获取这门课程的录像。"
          onRetry={() => void query.refetch()}
        />
      )}
      {query.data?.videos.length === 0 && (
        <EmptyState title="这门课程暂无录像" description="视频系统当前没有返回可下载录像。" />
      )}
      {query.data !== undefined && query.data.videos.length > 0 && (
        <section className="resource-list" aria-label="录像列表">
          {query.data.videos.map((video) => (
            <VideoCard key={video.id} video={video} courseHandle={courseHandle} />
          ))}
        </section>
      )}
    </AppShell>
  );
}

function navigationText(state: unknown, key: string): string | null {
  if (typeof state !== "object" || state === null || !(key in state)) return null;
  const value = (state as Record<string, unknown>)[key];
  return typeof value === "string" ? value : null;
}
