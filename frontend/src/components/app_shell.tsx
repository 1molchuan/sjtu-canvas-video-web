import { Link } from "react-router-dom";
import type { PropsWithChildren, ReactNode } from "react";

type AppShellProps = PropsWithChildren<{
  actions?: ReactNode;
  compact?: boolean;
}>;

export function AppShell({ actions, children, compact = false }: AppShellProps) {
  return (
    <div className={compact ? "app-shell app-shell--compact" : "app-shell"}>
      <header className="site-header">
        <Link className="wordmark" to="/" aria-label="Canvas Video Helper 首页">
          <span className="wordmark__index" aria-hidden="true">CVH</span>
          <span>Canvas Video Helper</span>
        </Link>
        <div className="site-header__actions">{actions}</div>
      </header>
      <main className="page-content">{children}</main>
      <footer className="site-footer">
        <p>非上海交通大学官方服务 · 服务器不保存课程视频</p>
        <Link to="/privacy">隐私与使用说明</Link>
      </footer>
    </div>
  );
}
