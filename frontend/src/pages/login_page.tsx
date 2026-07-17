import { QRCodeSVG } from "qrcode.react";
import { useMemo } from "react";
import { Navigate, useNavigate } from "react-router-dom";

import { AppShell } from "../components/app_shell";
import { LoadingState } from "../components/loading_state";
import { useAuth } from "../auth/auth_context";
import { useQrLogin, type QrLoginDependencies } from "../auth/use_qr_login";

export function LoginPage({ dependencies }: { dependencies?: QrLoginDependencies }) {
  const auth = useAuth();
  const navigate = useNavigate();
  const deps = useMemo(() => dependencies ?? browserDependencies(auth), [auth, dependencies]);
  const login = useQrLogin({
    deps,
    onAuthenticated: () => {
      void navigate("/courses", { replace: true });
    },
  });

  if (auth.state.status === "checking") {
    return <AppShell compact><LoadingState label="正在检查登录状态" /></AppShell>;
  }
  if (auth.state.status === "authenticated") {
    return <Navigate to="/courses" replace />;
  }

  return (
    <AppShell compact>
      <section className="login-layout">
        <div className="login-intro">
          <p className="eyebrow">受邀用户的私人课程录像下载工具</p>
          <h1>把课程录像，安全地带回你的设备。</h1>
          <p className="lede">使用你本人的 jAccount 权限登录。本站只做即时流式代理，不保存课程视频。</p>
          <ul className="trust-list">
            <li>非上海交通大学官方服务</li>
            <li>仅供受邀用户访问</li>
            <li>不保存账号密码或上游 Cookie</li>
          </ul>
        </div>
        <LoginCard login={login} />
      </section>
    </AppShell>
  );
}

function LoginCard({ login }: { login: ReturnType<typeof useQrLogin> }) {
  const state = login.state;
  return (
    <section className="login-card" aria-labelledby="login-card-title">
      <p className="folio">01 / 身份确认</p>
      <h2 id="login-card-title">jAccount 扫码登录</h2>
      <div className="qr-stage">
        {state.type === "waiting" ? (
          <>
            <QRCodeSVG value={state.qrUrl} size={220} level="M" role="img" aria-label="jAccount 登录二维码" />
            <a href={state.qrUrl} target="_blank" rel="noopener noreferrer">在新页面打开登录确认链接</a>
          </>
        ) : (
          <span className="qr-placeholder" aria-hidden="true">CVH</span>
        )}
      </div>
      <p className="login-status" aria-live="polite">{loginStatus(state.type)}</p>
      {(state.type === "rejected" || state.type === "error") && (
        <p className="inline-error" role="alert">
          {state.type === "rejected" ? "当前账号不在邀请名单。" : state.error.message}
        </p>
      )}
      <div className="login-actions">
        <button className="button button--primary" type="button" disabled={login.active} onClick={() => void login.start()}>
          {state.type === "expired" ? "重新生成二维码" : "开始扫码登录"}
        </button>
        {login.active && <button className="button button--quiet" type="button" onClick={login.cancel}>取消本次登录</button>}
      </div>
    </section>
  );
}

function loginStatus(type: ReturnType<typeof useQrLogin>["state"]["type"]): string {
  const messages: Record<typeof type, string> = {
    idle: "尚未开始",
    starting: "正在准备登录",
    waiting: "请使用 jAccount / 交我办完成扫码",
    scanned: "已扫码，等待确认",
    "authenticating": "正在建立 Canvas 会话",
    "completing-session": "正在完成本站登录",
    authenticated: "登录成功",
    rejected: "账号未受邀请",
    expired: "二维码已过期",
    error: "登录失败",
  };
  return messages[type];
}

function browserDependencies(auth: ReturnType<typeof useAuth>): QrLoginDependencies {
  return {
    startLogin: auth.authApi.startLogin,
    claimSession: auth.refresh,
    openEvents: (url) => new EventSource(url),
  };
}
