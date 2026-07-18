import { useMemo, useState } from "react";

import { PublicApiError } from "../api/client";
import { createSubtitleApi, type SubtitleApi } from "../api/subtitles";
import { useAuth } from "../auth/auth_context";

type SubtitleDownloadButtonProps = {
  courseHandle: string;
  videoHandle: string;
  videoName: string;
  api?: SubtitleApi;
};

type SubtitleStatus =
  | { type: "idle" }
  | { type: "downloading" }
  | { type: "completed" }
  | { type: "error"; message: string; requestId?: string };

export function SubtitleDownloadButton(props: SubtitleDownloadButtonProps) {
  const auth = useAuth();
  const api = useMemo(() => props.api ?? createSubtitleApi(auth.apiClient), [auth.apiClient, props.api]);
  const [status, setStatus] = useState<SubtitleStatus>({ type: "idle" });

  const download = async () => {
    setStatus({ type: "downloading" });
    try {
      await api.download(props.courseHandle, props.videoHandle, props.videoName);
      setStatus({ type: "completed" });
    } catch (error) {
      setStatus(subtitleError(error));
    }
  };

  return (
    <div className="subtitle-download">
      <div>
        <p className="eyebrow">字幕</p>
        <h2>识别字幕</h2>
        <p>若录像系统已生成字幕，可下载为 UTF-8 SRT 文件。</p>
      </div>
      <button
        className="button button--secondary"
        type="button"
        disabled={status.type === "downloading"}
        onClick={() => void download()}
      >
        {status.type === "downloading" ? "正在获取字幕" : "下载字幕"}
      </button>
      <SubtitleStatusMessage status={status} />
    </div>
  );
}

function SubtitleStatusMessage({ status }: { status: SubtitleStatus }) {
  if (status.type === "completed") {
    return <p className="download-status" aria-live="polite">字幕下载已开始。</p>;
  }
  if (status.type !== "error") return null;
  return (
    <p className="download-status download-status--error" role="alert">
      {status.message}
      {status.requestId !== undefined && <> 请求编号：{status.requestId}</>}
    </p>
  );
}

function subtitleError(error: unknown): SubtitleStatus {
  if (error instanceof PublicApiError) {
    return { type: "error", message: error.message, requestId: error.requestId };
  }
  return { type: "error", message: "字幕下载失败，请稍后重试。" };
}
