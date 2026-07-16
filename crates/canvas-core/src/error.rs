use serde::Serialize;
use thiserror::Error;

use crate::client::UpstreamPurpose;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ProtocolError {
    #[error("HTTP client construction failed")]
    HttpClientBuildFailed,
    #[error("in-memory Cookie Store is unavailable")]
    CookieStoreUnavailable,
    #[error("jAccount UUID is unavailable or ambiguous")]
    JAccountUuidUnavailable,
    #[error("jAccount UUID request failed")]
    JAccountUuidRequestFailed,
    #[error("jAccount WebSocket connection failed")]
    JAccountWebSocketConnect,
    #[error("jAccount WebSocket closed before login")]
    JAccountWebSocketClosed,
    #[error("jAccount login timed out")]
    JAccountLoginTimeout,
    #[error("jAccount login was cancelled")]
    JAccountLoginCancelled,
    #[error("jAccount express login failed")]
    JAccountExpressLoginFailed,
    #[error("jAccount authentication Cookie is missing")]
    JAccountCookieMissing,
    #[error("jAccount WebSocket message is invalid")]
    JAccountMessageInvalid,
    #[error("jAccount QR update payload is incomplete")]
    JAccountQrPayloadMissing,
    #[error("upstream URL rejected for {purpose:?}: {reason}")]
    InvalidUpstreamUrl {
        purpose: UpstreamPurpose,
        reason: &'static str,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CoreErrorCode {
    JAccountUnavailable,
    JAccountLoginFailed,
    CanvasLoginFailed,
    CanvasCourseDiscoveryUnavailable,
    CourseAccessDenied,
    LtiLaunchFailed,
    VideoTokenFailed,
    VideoListFailed,
    VideoInfoFailed,
    InvalidUpstreamHost,
    UpstreamUnavailable,
}

#[derive(Debug, Error)]
#[error("{code:?}: {message}")]
pub struct CoreError {
    pub code: CoreErrorCode,
    pub message: String,
}

impl CoreError {
    pub fn new(code: CoreErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}
