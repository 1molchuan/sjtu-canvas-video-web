import { PublicApiError } from "../api/client";

type ErrorNoticeProps = {
  error: Error;
  title?: string;
  onRetry?: () => void;
};

export function ErrorNotice({ error, title = "请求未能完成", onRetry }: ErrorNoticeProps) {
  const publicError = error instanceof PublicApiError ? error : null;
  return (
    <section className="error-notice" role="alert">
      <p className="eyebrow">发生错误</p>
      <h2>{title}</h2>
      <p>{publicError?.message ?? "服务暂时不可用，请稍后重试。"}</p>
      {publicError?.requestId !== undefined && (
        <p className="request-id">排错编号：{publicError.requestId}</p>
      )}
      {onRetry !== undefined && (
        <button className="button button--secondary" type="button" onClick={onRetry}>
          重试
        </button>
      )}
    </section>
  );
}
