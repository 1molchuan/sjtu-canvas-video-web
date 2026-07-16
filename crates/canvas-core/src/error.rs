use serde::Serialize;
use thiserror::Error;

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
