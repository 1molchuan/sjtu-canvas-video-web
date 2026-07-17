import { useMemo, useState } from "react";

import { PublicApiError } from "../api/client";
import {
  browserDirectDownload,
  DirectDownloadHttpError,
  DirectDownloadUnsupportedError,
  type DirectDownloadAdapter,
  type DirectDownloadFile,
} from "../api/direct_download";
import { createDownloadApi, startNativeDownload, type DownloadApi } from "../api/downloads";
import type { VideoTrack } from "../api/schemas";
import { useAuth } from "../auth/auth_context";

type DownloadStatus = "idle" | "selecting" | "issuing" | "downloading" | "started" | "completed";

export type TrackDownloadProps = {
  courseHandle: string;
  videoHandle: string;
  track: VideoTrack;
  index: number;
  downloads?: DownloadApi;
  startDownload?: (url: string) => void;
  directDownload?: DirectDownloadAdapter;
};

export function TrackDownloadButton(props: TrackDownloadProps) {
  const { session, apiClient } = useAuth();
  const downloads = useMemo(() => props.downloads ?? createDownloadApi(apiClient), [apiClient, props.downloads]);
  const [status, setStatus] = useState<DownloadStatus>("idle");
  const [error, setError] = useState<string | null>(null);
  const label = trackLabel(props.track.kind, props.index);
  const busy = status === "selecting" || status === "issuing" || status === "downloading";

  const download = async () => {
    if (session === null || busy) return;
    setError(null);
    try {
      const directFile = await selectFileIfNeeded(props, session.download_delivery, setStatus);
      if (session.download_delivery === "direct_stream" && directFile === null) return setStatus("idle");
      setStatus("issuing");
      const ticket = await downloads.issueTicket(ticketRequest(props, session.csrf_token));
      await deliverTicket(props, ticket.download_url, directFile, setStatus);
    } catch (caught) {
      setStatus("idle");
      setError(downloadErrorMessage(caught));
    }
  };

  return (
    <article className="track-row">
      <TrackDescription track={props.track} label={label} />
      <div className="track-row__action">
        <button className="button button--primary" type="button" disabled={session === null || busy} onClick={() => void download()}>
          {busy ? statusLabel(status) : `下载${label}`}
        </button>
        <p className={error === null ? "download-status" : "download-status download-status--error"} aria-live="polite">
          {error ?? successMessage(status)}
        </p>
      </div>
    </article>
  );
}

function TrackDescription({ track, label }: { track: VideoTrack; label: string }) {
  return (
    <div>
      <p className="track-row__kind">{track.kind === "unknown" ? "类型未识别" : "可下载轨道"}</p>
      <h2>{label}</h2>
      <p className="track-row__filename">{track.suggested_filename}</p>
    </div>
  );
}

async function selectFileIfNeeded(
  props: TrackDownloadProps,
  mode: "native_navigation" | "direct_stream",
  setStatus: (status: DownloadStatus) => void,
): Promise<DirectDownloadFile | null> {
  if (mode === "native_navigation") return null;
  setStatus("selecting");
  return (props.directDownload ?? browserDirectDownload).selectFile(props.track.suggested_filename);
}

async function deliverTicket(
  props: TrackDownloadProps,
  downloadUrl: string,
  directFile: DirectDownloadFile | null,
  setStatus: (status: DownloadStatus) => void,
): Promise<void> {
  if (directFile === null) {
    (props.startDownload ?? startNativeDownload)(downloadUrl);
    return setStatus("started");
  }
  setStatus("downloading");
  await (props.directDownload ?? browserDirectDownload).stream(downloadUrl, directFile);
  setStatus("completed");
}

function ticketRequest(props: TrackDownloadProps, csrfToken: string) {
  return {
    courseHandle: props.courseHandle,
    videoHandle: props.videoHandle,
    trackHandle: props.track.id,
    csrfToken,
  };
}

function trackLabel(kind: VideoTrack["kind"], index: number): string {
  if (kind === "camera") return "摄像画面";
  if (kind === "screen") return "屏幕画面";
  if (kind === "mixed") return "混合画面";
  return `视频轨道 ${String(index + 1)}`;
}

function statusLabel(status: DownloadStatus): string {
  if (status === "selecting") return "请选择保存位置";
  if (status === "downloading") return "正在直连下载";
  return "正在准备下载";
}

function successMessage(status: DownloadStatus): string {
  if (status === "started") return "下载已开始，可在浏览器下载列表中查看。";
  if (status === "completed") return "直连下载已完成。";
  return "";
}

function downloadErrorMessage(error: unknown): string {
  if (error instanceof DirectDownloadUnsupportedError) return "客户端直连下载需要最新版 Chrome 或 Edge。";
  if (error instanceof DirectDownloadHttpError && error.status === 429) return "已有下载正在进行，请稍后再试。";
  if (error instanceof DirectDownloadHttpError && error.status === 410) return "下载凭据已过期，请再次点击下载。";
  if (!(error instanceof PublicApiError)) return "下载失败，请重试。";
  if (error.status === 429) return "已有下载正在进行，请稍后再试。";
  if (error.status === 410) return "下载凭据已过期，请再次点击下载。";
  return error.message;
}
