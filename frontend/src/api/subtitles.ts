import type { ApiClient } from "./client";
import { encodeHandle } from "./courses";

const SUBTITLE_CONTENT_TYPE = "application/x-subrip";
const MAX_FILENAME_CHARS = 120;

export type SubtitleSaver = (blob: Blob, filename: string) => void;

export type SubtitleApi = {
  download: (courseHandle: string, videoHandle: string, videoName: string) => Promise<void>;
};

export function createSubtitleApi(
  client: ApiClient,
  save: SubtitleSaver = saveSubtitleBlob,
): SubtitleApi {
  return {
    async download(courseHandle, videoHandle, videoName) {
      const path = subtitlePath(courseHandle, videoHandle);
      const blob = await client.getBlob(path, SUBTITLE_CONTENT_TYPE);
      save(blob, subtitleFilename(videoName));
    },
  };
}

function subtitlePath(courseHandle: string, videoHandle: string): string {
  return `/api/courses/${encodeHandle(courseHandle)}/videos/${encodeHandle(videoHandle)}/subtitle`;
}

function subtitleFilename(videoName: string): string {
  const stemLimit = MAX_FILENAME_CHARS - ".srt".length;
  const stem = Array.from(videoName)
    .slice(0, stemLimit)
    .map((character) => (/^[\p{L}\p{N} _-]$/u.test(character) ? character : "_"))
    .join("")
    .replace(/^[_ .]+|[_ .]+$/g, "");
  return `${stem.length > 0 ? stem : "subtitle"}.srt`;
}

function saveSubtitleBlob(blob: Blob, filename: string): void {
  const url = URL.createObjectURL(blob);
  const anchor = document.createElement("a");
  anchor.href = url;
  anchor.download = filename;
  anchor.hidden = true;
  document.body.append(anchor);
  anchor.click();
  anchor.remove();
  URL.revokeObjectURL(url);
}
