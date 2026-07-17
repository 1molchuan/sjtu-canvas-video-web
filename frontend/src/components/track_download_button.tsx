import { useMemo, useState } from "react";

import { PublicApiError } from "../api/client";
import { createDownloadApi, startNativeDownload, type DownloadApi } from "../api/downloads";
import type { VideoTrack } from "../api/schemas";
import { useAuth } from "../auth/auth_context";

type TrackDownloadProps = {
  courseHandle: string;
  videoHandle: string;
  track: VideoTrack;
  index: number;
  downloads?: DownloadApi;
  startDownload?: (url: string) => void;
};

export function TrackDownloadButton(props: TrackDownloadProps) {
  const { session, apiClient } = useAuth();
  const downloads = useMemo(
    () => props.downloads ?? createDownloadApi(apiClient),
    [apiClient, props.downloads],
  );
  const [status, setStatus] = useState<"idle" | "issuing" | "started">("idle");
  const [error, setError] = useState<string | null>(null);
  const label = trackLabel(props.track.kind, props.index);

  const download = async () => {
    if (session === null || status === "issuing") return;
    setStatus("issuing");
    setError(null);
    try {
      const ticket = await downloads.issueTicket({
        courseHandle: props.courseHandle,
        videoHandle: props.videoHandle,
        trackHandle: props.track.id,
        csrfToken: session.csrf_token,
      });
      (props.startDownload ?? startNativeDownload)(ticket.download_url);
      setStatus("started");
    } catch (caught) {
      setStatus("idle");
      setError(downloadErrorMessage(caught));
    }
  };

  return (
    <article className="track-row">
      <div>
        <p className="track-row__kind">{props.track.kind === "unknown" ? "类型未识别" : "可下载轨道"}</p>
        <h2>{label}</h2>
        <p className="track-row__filename">{props.track.suggested_filename}</p>
      </div>
      <div className="track-row__action">
        <button
          className="button button--primary"
          type="button"
          disabled={session === null || status === "issuing"}
          onClick={() => void download()}
        >
          {status === "issuing" ? "正在准备下载" : `下载${label}`}
        </button>
        <p className={error === null ? "download-status" : "download-status download-status--error"} aria-live="polite">
          {error ?? (status === "started" ? "下载已开始，可在浏览器下载列表中查看。" : "")}
        </p>
      </div>
    </article>
  );
}

function trackLabel(kind: VideoTrack["kind"], index: number): string {
  if (kind === "camera") return "摄像画面";
  if (kind === "screen") return "屏幕画面";
  if (kind === "mixed") return "混合画面";
  return `视频轨道 ${String(index + 1)}`;
}

function downloadErrorMessage(error: unknown): string {
  if (!(error instanceof PublicApiError)) return "下载准备失败，请重试。";
  if (error.status === 429) return "已有下载正在进行，请稍后再试。";
  if (error.status === 410) return "下载凭据已过期，请再次点击下载。";
  return error.message;
}
