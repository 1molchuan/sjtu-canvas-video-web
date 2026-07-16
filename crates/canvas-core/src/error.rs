use serde::Serialize;
use thiserror::Error;

use crate::client::UpstreamPurpose;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ProtocolError {
    #[error("HTTP client construction failed")]
    HttpClientBuildFailed,
    #[error("upstream response body could not be read")]
    UpstreamBodyReadFailed,
    #[error("upstream response exceeded the configured limit")]
    UpstreamResponseTooLarge,
    #[error("in-memory Cookie Store is unavailable")]
    CookieStoreUnavailable,
    #[error("host-scoped Cookie could not be inserted")]
    CookieInsertFailed,
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
    #[error("stable user identity is unavailable")]
    IdentityUnavailable,
    #[error("Canvas login failed")]
    CanvasLoginFailed,
    #[error("Canvas login returned to the identity provider")]
    CanvasRedirectedToLogin,
    #[error("Canvas login redirect is missing")]
    CanvasRedirectMissing,
    #[error("Canvas login exceeded the redirect limit")]
    CanvasRedirectLimitExceeded,
    #[error("Canvas Session Cookie is missing")]
    CanvasSessionCookieMissing,
    #[error("Canvas course discovery request was rejected")]
    CanvasCourseDiscoveryRejected,
    #[error("Canvas course discovery response is unsupported")]
    CanvasCourseDiscoveryUnsupported,
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
