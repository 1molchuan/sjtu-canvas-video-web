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
    #[error("Canvas external tool page request failed")]
    ExternalToolPageFailed,
    #[error("OIDC initiation form is missing")]
    OidcFormMissing,
    #[error("OIDC initiation form action is invalid")]
    OidcActionInvalid,
    #[error("LTI authorization form is missing")]
    LtiFormMissing,
    #[error("LTI authorization form action is invalid")]
    LtiActionInvalid,
    #[error("LTI launch failed")]
    LtiLaunchFailed,
    #[error("LTI redirect is missing")]
    LtiRedirectMissing,
    #[error("LTI redirect is invalid")]
    LtiRedirectInvalid,
    #[error("LTI tokenId is missing, empty, or ambiguous")]
    TokenIdMissing,
    #[error("video token exchange failed")]
    VideoTokenExchangeFailed,
    #[error("video-system course ID is missing")]
    VideoCourseIdMissing,
    #[error("upstream protocol response changed")]
    UpstreamChanged,
    #[error("video course token has expired")]
    VideoTokenExpired,
    #[error("video list request failed")]
    VideoListFailed,
    #[error("video detail request failed")]
    VideoDetailFailed,
    #[error("video detail contains no usable track")]
    VideoTrackMissing,
    #[error("video Range probe failed")]
    RangeProbeFailed,
    #[error("video upstream rejected or changed Range semantics")]
    UpstreamRangeRejected,
    #[error("resolved upstream host is not allowed")]
    InvalidUpstreamHost,
    #[error("upstream URL rejected for {purpose:?}: {reason}")]
    InvalidUpstreamUrl {
        purpose: UpstreamPurpose,
        reason: &'static str,
    },
}
