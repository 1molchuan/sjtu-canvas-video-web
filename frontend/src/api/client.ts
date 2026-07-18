import type { ZodType } from "zod";

import { apiErrorEnvelopeSchema } from "./schemas";

const JSON_CONTENT_TYPE = "application/json";

export type ApiClientOptions = {
  fetcher?: typeof fetch;
  onUnauthorized?: () => void;
};

type RequestOptions = {
  method: "GET" | "POST";
  csrfToken?: string;
  accept?: string;
  body?: unknown;
};

export class PublicApiError extends Error {
  readonly status: number;
  readonly code: string;
  readonly requestId?: string;
  readonly retryAfter?: string;

  constructor(input: {
    status: number;
    code: string;
    message: string;
    requestId?: string;
    retryAfter?: string;
  }) {
    super(input.message);
    this.name = "PublicApiError";
    this.status = input.status;
    this.code = input.code;
    this.requestId = input.requestId;
    this.retryAfter = input.retryAfter;
  }
}

export class ApiClient {
  private readonly fetcher: typeof fetch;
  private readonly onUnauthorized?: () => void;

  constructor(options: ApiClientOptions = {}) {
    this.fetcher = options.fetcher ?? window.fetch.bind(window);
    this.onUnauthorized = options.onUnauthorized;
  }

  get<T>(path: string, schema: ZodType<T>): Promise<T> {
    return this.request(path, schema, { method: "GET" });
  }

  post<T>(path: string, schema: ZodType<T>, csrfToken?: string): Promise<T> {
    return this.request(path, schema, { method: "POST", csrfToken });
  }

  postJson<T>(path: string, schema: ZodType<T>, body: unknown): Promise<T> {
    return this.request(path, schema, { method: "POST", body });
  }

  async postNoContent(path: string, csrfToken?: string): Promise<void> {
    const response = await this.fetch(path, { method: "POST", csrfToken });
    if (response.status !== 204) {
      throw invalidResponse(response.status);
    }
  }

  async getBlob(path: string, expectedContentType: string): Promise<Blob> {
    const response = await this.fetch(path, { method: "GET", accept: expectedContentType });
    const contentType = response.headers.get("content-type")?.toLowerCase() ?? "";
    if (!contentType.includes(expectedContentType.toLowerCase())) {
      throw invalidResponse(response.status);
    }
    return response.blob();
  }

  private async request<T>(
    path: string,
    schema: ZodType<T>,
    options: RequestOptions,
  ): Promise<T> {
    const response = await this.fetch(path, options);
    if (!isJson(response)) {
      throw invalidResponse(response.status);
    }
    const payload: unknown = await response.json();
    const parsed = schema.safeParse(payload);
    if (!parsed.success) {
      throw invalidResponse(response.status);
    }
    return parsed.data;
  }

  private async fetch(path: string, options: RequestOptions): Promise<Response> {
    assertApiPath(path);
    let response: Response;
    try {
      response = await this.fetcher(path, buildRequest(options));
    } catch {
      throw new PublicApiError({
        status: 0,
        code: "NETWORK_ERROR",
        message: "网络连接失败，请检查连接后重试。",
      });
    }
    if (response.status === 401) {
      this.onUnauthorized?.();
    }
    if (!response.ok) {
      throw await responseError(response);
    }
    return response;
  }
}

function buildRequest(options: RequestOptions): RequestInit {
  const headers = new Headers({ Accept: options.accept ?? JSON_CONTENT_TYPE });
  if (options.csrfToken !== undefined) {
    headers.set("X-CSRF-Token", options.csrfToken);
  }
  if (options.body !== undefined) {
    headers.set("Content-Type", JSON_CONTENT_TYPE);
  }
  return {
    method: options.method,
    credentials: "same-origin",
    headers,
    body: options.body === undefined ? undefined : JSON.stringify(options.body),
  };
}

async function responseError(response: Response): Promise<PublicApiError> {
  if (isJson(response)) {
    const parsed = apiErrorEnvelopeSchema.safeParse(await response.json());
    if (parsed.success) {
      return new PublicApiError({
        status: response.status,
        code: parsed.data.error.code,
        message: parsed.data.error.message,
        requestId: parsed.data.error.request_id,
        retryAfter: response.headers.get("retry-after") ?? undefined,
      });
    }
  }
  return new PublicApiError({
    status: response.status,
    code: "HTTP_ERROR",
    message: "请求失败，请稍后重试。",
    requestId: response.headers.get("x-request-id") ?? undefined,
  });
}

function invalidResponse(status: number): PublicApiError {
  return new PublicApiError({
    status,
    code: "INVALID_RESPONSE",
    message: "服务返回了无法识别的数据。",
  });
}

function isJson(response: Response): boolean {
  return response.headers.get("content-type")?.toLowerCase().includes(JSON_CONTENT_TYPE) ?? false;
}

function assertApiPath(path: string): void {
  if (!path.startsWith("/api/")) {
    throw new PublicApiError({
      status: 0,
      code: "INVALID_API_PATH",
      message: "请求地址无效。",
    });
  }
}
