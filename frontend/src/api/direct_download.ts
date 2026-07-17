const DOWNLOAD_PATH_PREFIX = "/api/download/";

type SaveFilePickerOptions = {
  suggestedName: string;
  types: { description: string; accept: Record<string, string[]> }[];
};

export type DirectDownloadFile = {
  createWritable: () => Promise<WritableStream<Uint8Array>>;
};

type FilePickerWindow = Window & {
  showSaveFilePicker?: (options: SaveFilePickerOptions) => Promise<DirectDownloadFile>;
};

export type DirectDownloadAdapter = {
  selectFile: (filename: string) => Promise<DirectDownloadFile | null>;
  stream: (downloadUrl: string, file: DirectDownloadFile) => Promise<void>;
};

export class DirectDownloadUnsupportedError extends Error {
  constructor() {
    super("direct download requires the File System Access API");
  }
}

export class DirectDownloadHttpError extends Error {
  constructor(readonly status: number) {
    super(`direct download returned HTTP ${String(status)}`);
  }
}

export async function selectDirectDownloadFile(filename: string): Promise<DirectDownloadFile | null> {
  const picker = (window as FilePickerWindow).showSaveFilePicker;
  if (picker === undefined) throw new DirectDownloadUnsupportedError();
  try {
    return await picker({
      suggestedName: filename,
      types: [{ description: "MP4 视频", accept: { "video/mp4": [".mp4"] } }],
    });
  } catch (error) {
    if (error instanceof DOMException && error.name === "AbortError") return null;
    throw error;
  }
}

export async function streamDirectDownload(downloadUrl: string, file: DirectDownloadFile): Promise<void> {
  validateDownloadPath(downloadUrl);
  const response = await fetch(downloadUrl, {
    credentials: "same-origin",
    cache: "no-store",
    redirect: "follow",
  });
  if (!response.ok) throw new DirectDownloadHttpError(response.status);
  if (response.body === null) throw new Error("direct download response has no body");
  const writable = await file.createWritable();
  await response.body.pipeTo(writable);
}

export const browserDirectDownload: DirectDownloadAdapter = {
  selectFile: selectDirectDownloadFile,
  stream: streamDirectDownload,
};

function validateDownloadPath(downloadUrl: string): void {
  if (!downloadUrl.startsWith(DOWNLOAD_PATH_PREFIX)) {
    throw new Error("download path must be a same-origin ticket route");
  }
}
