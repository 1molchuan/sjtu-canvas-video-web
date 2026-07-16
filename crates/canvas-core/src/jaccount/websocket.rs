use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use secrecy::{ExposeSecret, SecretString};
use tokio::{sync::mpsc, time::Instant};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tokio_util::sync::CancellationToken;

use crate::{
    client::{ProtocolContext, UpstreamPurpose, validate_upstream_url},
    error::ProtocolError,
};

use super::{QrEvent, build_qr_url, parse_qr_message, parse_uuid_from_html};

const UPDATE_MESSAGE: &str = r#"{"type":"UPDATE_QR_CODE"}"#;
const JA_AUTH_COOKIE: &str = "JAAuthCookie";

pub struct QrLoginOptions {
    pub timeout: Duration,
    pub refresh_interval: Duration,
    pub cancellation: CancellationToken,
}

#[derive(Debug)]
pub enum QrLoginProgress {
    QrReady { url: SecretString },
    Expired,
    UnknownEvent { event_type: String },
}

#[derive(Debug)]
pub struct JAccountSession {
    pub cookie_names: Vec<String>,
    pub ja_auth_cookie: SecretString,
}

struct LoginEventHandler<'a> {
    context: &'a ProtocolContext,
    uuid: &'a SecretString,
    progress: mpsc::UnboundedSender<QrLoginProgress>,
}

pub async fn login_with_qr(
    context: &ProtocolContext,
    options: QrLoginOptions,
    progress: mpsc::UnboundedSender<QrLoginProgress>,
) -> Result<JAccountSession, ProtocolError> {
    let uuid = fetch_uuid(context).await?;
    let websocket_url = context
        .endpoints
        .websocket_base
        .join(uuid.expose_secret())
        .map_err(|_| ProtocolError::JAccountWebSocketConnect)?;
    validate_upstream_url(&websocket_url, UpstreamPurpose::JAccount, &context.policy)?;
    let (mut socket, _) = connect_async(websocket_url)
        .await
        .map_err(|_| ProtocolError::JAccountWebSocketConnect)?;
    socket
        .send(Message::Text(UPDATE_MESSAGE.into()))
        .await
        .map_err(|_| ProtocolError::JAccountWebSocketConnect)?;

    let handler = LoginEventHandler {
        context,
        uuid: &uuid,
        progress,
    };
    let result = wait_for_login(&mut socket, options, &handler).await;
    let _ = socket.close(None).await;
    result?;
    express_login(context, &uuid).await
}

async fn fetch_uuid(context: &ProtocolContext) -> Result<SecretString, ProtocolError> {
    validate_upstream_url(
        &context.endpoints.my_info,
        UpstreamPurpose::MySjtu,
        &context.policy,
    )?;
    let response = context
        .client
        .get(context.endpoints.my_info.clone())
        .send()
        .await
        .map_err(|_| ProtocolError::JAccountUuidRequestFailed)?;
    if !response.status().is_success() {
        return Err(ProtocolError::JAccountUuidRequestFailed);
    }
    let html = response
        .text()
        .await
        .map_err(|_| ProtocolError::JAccountUuidRequestFailed)?;
    parse_uuid_from_html(&html)
}

async fn express_login(
    context: &ProtocolContext,
    uuid: &SecretString,
) -> Result<JAccountSession, ProtocolError> {
    validate_upstream_url(
        &context.endpoints.express_login,
        UpstreamPurpose::JAccount,
        &context.policy,
    )?;
    let response = context
        .client
        .get(context.endpoints.express_login.clone())
        .query(&[("uuid", uuid.expose_secret())])
        .send()
        .await
        .map_err(|_| ProtocolError::JAccountExpressLoginFailed)?;
    if !response.status().is_success() {
        return Err(ProtocolError::JAccountExpressLoginFailed);
    }
    let origin = context.endpoints.jaccount_origin();
    let ja_auth_cookie = context
        .cookie_value(&origin, JA_AUTH_COOKIE)?
        .ok_or(ProtocolError::JAccountCookieMissing)?;
    Ok(JAccountSession {
        cookie_names: context.cookie_names(&origin)?,
        ja_auth_cookie,
    })
}

async fn wait_for_login<S>(
    socket: &mut tokio_tungstenite::WebSocketStream<S>,
    options: QrLoginOptions,
    handler: &LoginEventHandler<'_>,
) -> Result<(), ProtocolError>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    let deadline = tokio::time::sleep_until(Instant::now() + options.timeout);
    tokio::pin!(deadline);
    let mut refresh = tokio::time::interval(options.refresh_interval);
    refresh.tick().await;
    loop {
        tokio::select! {
            _ = &mut deadline => return Err(ProtocolError::JAccountLoginTimeout),
            _ = options.cancellation.cancelled() => return Err(ProtocolError::JAccountLoginCancelled),
            _ = refresh.tick() => send_refresh(socket).await?,
            message = socket.next() => {
                if handle_message(socket, message, handler).await? {
                    return Ok(());
                }
            }
        }
    }
}

async fn send_refresh<S>(
    socket: &mut tokio_tungstenite::WebSocketStream<S>,
) -> Result<(), ProtocolError>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    socket
        .send(Message::Text(UPDATE_MESSAGE.into()))
        .await
        .map_err(|_| ProtocolError::JAccountWebSocketClosed)
}

async fn handle_message<S>(
    socket: &mut tokio_tungstenite::WebSocketStream<S>,
    message: Option<Result<Message, tokio_tungstenite::tungstenite::Error>>,
    handler: &LoginEventHandler<'_>,
) -> Result<bool, ProtocolError>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    match message {
        Some(Ok(Message::Text(text))) => handler.handle_text(text.as_ref()),
        Some(Ok(Message::Ping(payload))) => {
            socket
                .send(Message::Pong(payload))
                .await
                .map_err(|_| ProtocolError::JAccountWebSocketClosed)?;
            Ok(false)
        }
        Some(Ok(Message::Close(_))) | None | Some(Err(_)) => {
            Err(ProtocolError::JAccountWebSocketClosed)
        }
        Some(Ok(_)) => Ok(false),
    }
}

impl LoginEventHandler<'_> {
    fn handle_text(&self, text: &str) -> Result<bool, ProtocolError> {
        match parse_qr_message(text)? {
            QrEvent::Update(payload) => {
                let url = build_qr_url(&self.context.endpoints.qr_confirm, self.uuid, &payload)?;
                self.progress
                    .send(QrLoginProgress::QrReady { url })
                    .map_err(|_| ProtocolError::JAccountLoginCancelled)?;
                Ok(false)
            }
            QrEvent::Login => Ok(true),
            QrEvent::Expired => {
                let _ = self.progress.send(QrLoginProgress::Expired);
                Err(ProtocolError::JAccountQrExpired)
            }
            QrEvent::Unknown { event_type } => {
                let _ = self
                    .progress
                    .send(QrLoginProgress::UnknownEvent { event_type });
                Ok(false)
            }
        }
    }
}
