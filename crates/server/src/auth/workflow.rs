use std::{sync::Arc, time::Duration};

use canvas_core::{
    ProtocolError,
    client::ProtocolContext,
    jaccount::{QrLoginOptions, QrLoginProgress},
};
use tokio::sync::mpsc;

use super::{
    login::AuthenticatedLogin,
    pending::{BrowserQrUrl, LoginEvent, PendingLogin, PendingLoginState},
};
use crate::state::AppState;

const QR_REFRESH_SECONDS: u64 = 25;

struct LoginAttempt {
    context: ProtocolContext,
    options: QrLoginOptions,
}

pub fn spawn(state: AppState, pending: Arc<PendingLogin>) {
    tokio::spawn(async move {
        run(state, pending).await;
    });
}

async fn run(state: AppState, pending: Arc<PendingLogin>) {
    pending.publish(LoginEvent::Started);
    if pending
        .transition(PendingLoginState::WaitingForQr)
        .await
        .is_err()
    {
        return;
    }
    let context = match ProtocolContext::new(state.protocol_config()) {
        Ok(context) => context,
        Err(_) => return fail(&pending).await,
    };
    let attempt = LoginAttempt {
        context,
        options: login_options(&state, &pending),
    };
    let result = drive_login(&state, &pending, attempt).await;
    finish(&state, &pending, result).await;
}

fn login_options(state: &AppState, pending: &PendingLogin) -> QrLoginOptions {
    let timeout = Duration::from_secs(state.config().server.pending_login_ttl_minutes * 60);
    QrLoginOptions {
        timeout,
        refresh_interval: Duration::from_secs(QR_REFRESH_SECONDS),
        cancellation: pending.cancellation(),
    }
}

async fn drive_login(
    state: &AppState,
    pending: &PendingLogin,
    attempt: LoginAttempt,
) -> Result<AuthenticatedLogin, ProtocolError> {
    let (progress_tx, mut progress_rx) = mpsc::unbounded_channel();
    let timeout = Duration::from_secs(state.config().server.pending_login_ttl_minutes * 60);
    let provider = state.login_provider();
    let login = tokio::time::timeout(
        timeout,
        provider.authenticate(attempt.context, attempt.options, progress_tx),
    );
    let shutdown = state.shutdown();
    tokio::pin!(login);
    let result = loop {
        tokio::select! {
            result = &mut login => {
                break result.map_err(|_| ProtocolError::JAccountLoginTimeout)?;
            }
            Some(event) = progress_rx.recv() => handle_progress(pending, event).await,
            _ = shutdown.cancelled() => {
                break Err(ProtocolError::JAccountLoginCancelled);
            }
        }
    };
    while let Ok(event) = progress_rx.try_recv() {
        handle_progress(pending, event).await;
    }
    result
}

async fn handle_progress(pending: &PendingLogin, progress: QrLoginProgress) {
    match progress {
        QrLoginProgress::QrReady { url } => qr_ready(pending, url).await,
        QrLoginProgress::Scanned => scanned(pending).await,
        QrLoginProgress::Expired => expired(pending).await,
        QrLoginProgress::UnknownEvent { .. } => {}
    }
}

async fn qr_ready(pending: &PendingLogin, url: secrecy::SecretString) {
    if pending.state().await == PendingLoginState::WaitingForQr {
        let _ = pending.transition(PendingLoginState::WaitingForScan).await;
    }
    pending.publish(LoginEvent::Qr {
        url: BrowserQrUrl::new(url),
    });
}

async fn scanned(pending: &PendingLogin) {
    if pending
        .transition(PendingLoginState::Authenticating)
        .await
        .is_ok()
    {
        pending.publish(LoginEvent::Scanned);
        pending.publish(LoginEvent::Authenticating);
    }
}

async fn expired(pending: &PendingLogin) {
    if pending.transition(PendingLoginState::Expired).await.is_ok() {
        pending.publish(LoginEvent::Expired);
        pending.cancellation().cancel();
    }
}

async fn finish(
    state: &AppState,
    pending: &PendingLogin,
    result: Result<AuthenticatedLogin, ProtocolError>,
) {
    let Ok(login) = result else {
        if pending.state().await != PendingLoginState::Expired {
            fail(pending).await;
        }
        return;
    };
    if !state.whitelist().allows(&login.identity.stable_id) {
        let _ = pending.transition(PendingLoginState::Failed).await;
        pending.publish(LoginEvent::Rejected);
        return;
    }
    let _ = pending.complete(login).await;
}

async fn fail(pending: &PendingLogin) {
    let _ = pending.transition(PendingLoginState::Failed).await;
    pending.publish(LoginEvent::Error {
        code: "LOGIN_FAILED".to_owned(),
        message: "登录失败，请重新扫码。".to_owned(),
    });
}
