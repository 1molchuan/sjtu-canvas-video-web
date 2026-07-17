import { Link } from "react-router-dom";

import type { Video } from "../api/schemas";

type VideoCardProps = {
  video: Video;
  courseHandle: string;
};

export function VideoCard({ video, courseHandle }: VideoCardProps) {
  return (
    <article className="resource-row">
      <div>
        <p className="resource-row__date">{formatStartedAt(video.started_at)}</p>
        <h2>{video.name || "未命名录像"}</h2>
      </div>
      <Link
        className="button button--secondary"
        to={`/courses/${encodeURIComponent(courseHandle)}/videos/${encodeURIComponent(video.id)}`}
        state={{ videoName: video.name, startedAt: video.started_at }}
      >
        查看轨道
      </Link>
    </article>
  );
}

function formatStartedAt(value: string | null): string {
  if (value === null) return "时间信息暂缺";
  const date = new Date(value);
  if (Number.isNaN(date.valueOf())) return "时间信息暂缺";
  return new Intl.DateTimeFormat("zh-CN", {
    dateStyle: "medium",
    timeStyle: "short",
  }).format(date);
}
