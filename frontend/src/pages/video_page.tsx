import { useQuery } from "@tanstack/react-query";
import { useMemo } from "react";
import { Link, useLocation, useParams } from "react-router-dom";

import { createCourseApi, shouldRetryQuery, type CourseApi } from "../api/courses";
import type { DownloadApi } from "../api/downloads";
import { useAuth } from "../auth/auth_context";
import { AppShell } from "../components/app_shell";
import { EmptyState } from "../components/empty_state";
import { ErrorNotice } from "../components/error_notice";
import { LoadingState } from "../components/loading_state";
import { TrackDownloadButton } from "../components/track_download_button";

type VideoPageProps = {
  api?: CourseApi;
  downloads?: DownloadApi;
  startDownload?: (url: string) => void;
};

export function VideoPage(props: VideoPageProps) {
  const { courseHandle = "", videoHandle = "" } = useParams();
  const location = useLocation();
  const auth = useAuth();
  const courseApi = useMemo(() => props.api ?? createCourseApi(auth.apiClient), [auth.apiClient, props.api]);
  const query = useQuery({
    queryKey: ["video", courseHandle, videoHandle],
    queryFn: () => courseApi.getVideoDetail(courseHandle, videoHandle),
    enabled: auth.session !== null && courseHandle.length > 0 && videoHandle.length > 0,
    retry: shouldRetryQuery,
    retryDelay: 0,
  });
  const back = `/courses/${encodeURIComponent(courseHandle)}`;
  const startedAt = navigationText(location.state, "startedAt");

  return (
    <AppShell actions={<Link className="button button--quiet" to={back}>返回课程</Link>}>
      <header className="page-heading page-heading--with-rule">
        <p className="eyebrow">录像轨道</p>
        <h1>{query.data?.video.name ?? navigationText(location.state, "videoName") ?? "课程录像"}</h1>
        {startedAt !== null && <p>{formatTime(startedAt)}</p>}
      </header>
      {query.isPending && <LoadingState label="正在获取录像轨道" />}
      {query.error !== null && <ErrorNotice error={query.error} onRetry={() => void query.refetch()} />}
      {query.data?.video.tracks.length === 0 && (
        <EmptyState title="没有可下载轨道" description="视频系统没有返回可用的视频轨道。" />
      )}
      {query.data !== undefined && query.data.video.tracks.length > 0 && (
        <section className="track-list" aria-label="视频轨道列表">
          {query.data.video.tracks.map((track, index) => (
            <TrackDownloadButton
              key={track.id}
              courseHandle={courseHandle}
              videoHandle={videoHandle}
              track={track}
              index={index}
              downloads={props.downloads}
              startDownload={props.startDownload}
            />
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

function formatTime(value: string): string {
  const date = new Date(value);
  if (Number.isNaN(date.valueOf())) return "时间信息暂缺";
  return new Intl.DateTimeFormat("zh-CN", { dateStyle: "medium", timeStyle: "short" }).format(date);
}
