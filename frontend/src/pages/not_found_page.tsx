import { Link } from "react-router-dom";

import { AppShell } from "../components/app_shell";

export function NotFoundPage() {
  return (
    <AppShell compact>
      <section className="not-found">
        <p className="folio">404 / 未归档</p>
        <h1>页面不存在</h1>
        <p>这个地址没有对应的课程页面，或临时链接已经失效。</p>
        <Link className="button button--primary" to="/">返回首页</Link>
      </section>
    </AppShell>
  );
}
