import { Link } from "react-router-dom";

import { AppShell } from "../components/app_shell";

export function PrivacyPage() {
  return (
    <AppShell actions={<Link className="button button--quiet" to="/">返回应用</Link>}>
      <article className="prose-page">
        <header className="page-heading page-heading--with-rule">
          <p className="eyebrow">项目边界</p>
          <h1>隐私与使用说明</h1>
          <p>请在扫码前了解本站如何处理登录状态与课程录像。</p>
        </header>
        <section>
          <h2>非官方、受邀访问</h2>
          <p>这是非上海交通大学官方的私人项目，只允许维护者邀请的账号访问，也不代表学校、Canvas 或课程视频平台的官方合作。</p>
        </section>
        <section>
          <h2>登录信息只留在当前会话</h2>
          <p>本站使用你本人的 jAccount 与 Canvas 权限，不收集账号密码，也不会把上游 Cookie、LTI token 或视频 token 发送到浏览器。上游会话只保存在服务器内存中，登出、过期或服务重启后即销毁。</p>
        </section>
        <section>
          <h2>课程视频不落盘</h2>
          <p>录像由学校上游经过服务器即时流式传输到你的浏览器。服务器不缓存、不保存，也不上传课程视频到对象存储或第三方平台。</p>
        </section>
        <section>
          <h2>合理使用</h2>
          <p>下载内容只应由你本人在课程授权范围内合理使用。不得公开传播受课程访问控制保护的录像；出现异常时，请通过维护者私下提供的渠道联系。</p>
        </section>
      </article>
    </AppShell>
  );
}
