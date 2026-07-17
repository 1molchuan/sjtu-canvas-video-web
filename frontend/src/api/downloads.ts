import type { ApiClient } from "./client";
import { encodeHandle } from "./courses";
import { ticketResponseSchema, type TicketResponse } from "./schemas";

export type TicketRequest = {
  courseHandle: string;
  videoHandle: string;
  trackHandle: string;
  csrfToken: string;
};

export type DownloadApi = {
  issueTicket: (request: TicketRequest) => Promise<TicketResponse>;
};

export function createDownloadApi(client: ApiClient): DownloadApi {
  return {
    issueTicket: (request) =>
      client.post(ticketPath(request), ticketResponseSchema, request.csrfToken),
  };
}

export function startNativeDownload(downloadUrl: string): void {
  if (!downloadUrl.startsWith("/api/download/")) {
    throw new Error("invalid download path");
  }
  const anchor = document.createElement("a");
  anchor.href = downloadUrl;
  anchor.download = "";
  anchor.hidden = true;
  document.body.append(anchor);
  anchor.click();
  anchor.remove();
}

function ticketPath(request: TicketRequest): string {
  const course = encodeHandle(request.courseHandle);
  const video = encodeHandle(request.videoHandle);
  const track = encodeHandle(request.trackHandle);
  return `/api/courses/${course}/videos/${video}/tracks/${track}/ticket`;
}
