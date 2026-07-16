use canvas_core::ProtocolError;

use crate::report::ReportWriteError;

#[derive(Debug, thiserror::Error)]
pub enum CliError {
    #[error("real protocol validation is disabled")]
    RealModeDisabled,
    #[error("protocol operation failed: {0}")]
    Protocol(#[from] ProtocolError),
    #[error("QR code could not be rendered")]
    QrRender,
    #[error("validation timestamp could not be formatted")]
    Timestamp,
    #[error("no authorized video is available for inspection")]
    NoVideoAvailable,
    #[error("requested video is not present in the authorized course catalog")]
    RequestedVideoUnavailable,
    #[error("safe terminal output could not be produced")]
    Output,
    #[error("validation report output failed: {0}")]
    Report(#[from] ReportWriteError),
}

impl CliError {
    pub const fn class(&self) -> &'static str {
        match self {
            Self::RealModeDisabled => "real_mode_disabled",
            Self::Protocol(error) => protocol_error_class(error),
            Self::QrRender => "qr_render_failed",
            Self::Timestamp => "timestamp_failed",
            Self::NoVideoAvailable => "no_video_available",
            Self::RequestedVideoUnavailable => "requested_video_unavailable",
            Self::Output => "output_failed",
            Self::Report(_) => "report_write_failed",
        }
    }
}

const fn protocol_error_class(error: &ProtocolError) -> &'static str {
    match error {
        ProtocolError::JAccountUuidUnavailable | ProtocolError::JAccountUuidRequestFailed => {
            "jaccount_uuid_failed"
        }
        ProtocolError::JAccountWebSocketConnect
        | ProtocolError::JAccountWebSocketClosed
        | ProtocolError::JAccountLoginTimeout
        | ProtocolError::JAccountQrExpired
        | ProtocolError::JAccountLoginCancelled => "jaccount_websocket_failed",
        ProtocolError::JAccountExpressLoginFailed | ProtocolError::JAccountCookieMissing => {
            "jaccount_express_login_failed"
        }
        ProtocolError::CanvasLoginFailed
        | ProtocolError::CanvasRedirectedToLogin
        | ProtocolError::CanvasRedirectMissing
        | ProtocolError::CanvasRedirectLimitExceeded
        | ProtocolError::CanvasSessionCookieMissing => "canvas_login_failed",
        ProtocolError::IdentityUnavailable => "identity_unavailable",
        ProtocolError::CanvasCourseDiscoveryRejected
        | ProtocolError::CanvasCourseDiscoveryUnsupported => "course_discovery_failed",
        ProtocolError::VideoListFailed | ProtocolError::VideoTokenExpired => "video_list_failed",
        ProtocolError::VideoDetailFailed | ProtocolError::VideoTrackMissing => {
            "video_detail_failed"
        }
        ProtocolError::RangeProbeFailed | ProtocolError::UpstreamRangeRejected => {
            "range_probe_failed"
        }
        ProtocolError::InvalidUpstreamHost | ProtocolError::InvalidUpstreamUrl { .. } => {
            "invalid_upstream"
        }
        _ => "protocol_failed",
    }
}
