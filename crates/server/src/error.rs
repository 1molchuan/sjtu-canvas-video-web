use crate::middleware::request_id;
use axum::{
    Json,
    http::{HeaderValue, StatusCode, header},
    response::{IntoResponse, Response},
};
use serde::Serialize;

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum WebErrorCode {
    Unauthorized,
    SessionExpired,
    CsrfRejected,
    OriginRejected,
    UserNotAllowed,
    TooManyPendingLogins,
    QrStartRateLimited,
    InviteInvalid,
    InviteExpired,
    InviteAlreadyUsed,
    PendingLoginNotFound,
    PendingLoginExpired,
    CourseHandleInvalid,
    VideoHandleInvalid,
    TrackHandleInvalid,
    SubtitleNotFound,
    DownloadTicketInvalid,
    DownloadTicketExpired,
    DownloadTicketSessionMismatch,
    DownloadLimitExceeded,
    InvalidRange,
    MultipleRangesUnsupported,
    RequestTooLarge,
    ApiRouteNotFound,
    UpstreamRejectedRange,
    UpstreamUnavailable,
    Internal,
}

pub struct WebError {
    status: StatusCode,
    code: WebErrorCode,
    message: &'static str,
    retry_after: Option<HeaderValue>,
}

#[derive(Serialize)]
struct ErrorEnvelope {
    error: ErrorBody,
}

#[derive(Serialize)]
struct ErrorBody {
    code: WebErrorCode,
    message: &'static str,
    request_id: String,
}

impl WebError {
    pub fn origin_rejected() -> Self {
        Self::new(
            StatusCode::FORBIDDEN,
            WebErrorCode::OriginRejected,
            "请求来源校验失败。",
        )
    }

    pub fn too_many_pending() -> Self {
        Self::new(
            StatusCode::TOO_MANY_REQUESTS,
            WebErrorCode::TooManyPendingLogins,
            "当前扫码登录请求过多，请稍后重试。",
        )
    }

    pub fn qr_start_rate_limited() -> Self {
        let mut error = Self::new(
            StatusCode::TOO_MANY_REQUESTS,
            WebErrorCode::QrStartRateLimited,
            "扫码登录请求过于频繁，请稍后重试。",
        );
        error.retry_after = Some(HeaderValue::from_static("60"));
        error
    }

    pub fn invite_invalid() -> Self {
        Self::new(
            StatusCode::FORBIDDEN,
            WebErrorCode::InviteInvalid,
            "邀请链接无效。",
        )
    }

    pub fn invite_expired() -> Self {
        Self::new(
            StatusCode::GONE,
            WebErrorCode::InviteExpired,
            "邀请链接已过期。",
        )
    }

    pub fn invite_already_used() -> Self {
        Self::new(
            StatusCode::GONE,
            WebErrorCode::InviteAlreadyUsed,
            "邀请链接已使用或正在使用。",
        )
    }

    pub fn pending_not_found() -> Self {
        Self::new(
            StatusCode::NOT_FOUND,
            WebErrorCode::PendingLoginNotFound,
            "扫码登录状态不存在或已过期。",
        )
    }

    pub fn csrf_rejected() -> Self {
        Self::new(
            StatusCode::FORBIDDEN,
            WebErrorCode::CsrfRejected,
            "CSRF 校验失败，请刷新页面后重试。",
        )
    }

    pub fn unauthorized() -> Self {
        Self::new(
            StatusCode::UNAUTHORIZED,
            WebErrorCode::Unauthorized,
            "请先使用 jAccount 扫码登录。",
        )
    }

    pub fn session_expired() -> Self {
        Self::new(
            StatusCode::UNAUTHORIZED,
            WebErrorCode::SessionExpired,
            "登录状态已过期，请重新扫码登录。",
        )
    }

    pub fn invalid_course_handle() -> Self {
        Self::new(
            StatusCode::NOT_FOUND,
            WebErrorCode::CourseHandleInvalid,
            "课程不存在或访问句柄已过期。",
        )
    }

    pub fn invalid_video_handle() -> Self {
        Self::new(
            StatusCode::NOT_FOUND,
            WebErrorCode::VideoHandleInvalid,
            "录像不存在或访问句柄已过期。",
        )
    }

    pub fn invalid_track_handle() -> Self {
        Self::new(
            StatusCode::NOT_FOUND,
            WebErrorCode::TrackHandleInvalid,
            "视频轨道不存在或访问句柄已过期。",
        )
    }

    pub fn subtitle_missing() -> Self {
        Self::new(
            StatusCode::NOT_FOUND,
            WebErrorCode::SubtitleNotFound,
            "这段录像暂时没有可用字幕。",
        )
    }

    pub fn upstream_unavailable() -> Self {
        Self::new(
            StatusCode::BAD_GATEWAY,
            WebErrorCode::UpstreamUnavailable,
            "上游课程录像服务暂时不可用。",
        )
    }

    pub fn invalid_download_ticket() -> Self {
        Self::new(
            StatusCode::NOT_FOUND,
            WebErrorCode::DownloadTicketInvalid,
            "下载凭证无效。",
        )
    }

    pub fn expired_download_ticket() -> Self {
        Self::new(
            StatusCode::GONE,
            WebErrorCode::DownloadTicketExpired,
            "下载凭证已过期，请重新获取。",
        )
    }

    pub fn download_ticket_session_mismatch() -> Self {
        Self::new(
            StatusCode::FORBIDDEN,
            WebErrorCode::DownloadTicketSessionMismatch,
            "下载凭证不属于当前登录会话。",
        )
    }

    pub fn download_limit_exceeded() -> Self {
        let mut error = Self::new(
            StatusCode::TOO_MANY_REQUESTS,
            WebErrorCode::DownloadLimitExceeded,
            "同时下载数量已达到限制，请稍后重试。",
        );
        error.retry_after = Some(HeaderValue::from_static("5"));
        error
    }

    pub fn invalid_range() -> Self {
        Self::new(
            StatusCode::RANGE_NOT_SATISFIABLE,
            WebErrorCode::InvalidRange,
            "Range 请求格式无效。",
        )
    }

    pub fn multiple_ranges_unsupported() -> Self {
        Self::new(
            StatusCode::RANGE_NOT_SATISFIABLE,
            WebErrorCode::MultipleRangesUnsupported,
            "暂不支持多个 Range 区间。",
        )
    }

    pub fn upstream_rejected_range() -> Self {
        Self::new(
            StatusCode::BAD_GATEWAY,
            WebErrorCode::UpstreamRejectedRange,
            "上游服务未接受续传请求。",
        )
    }

    pub fn request_too_large() -> Self {
        Self::new(
            StatusCode::PAYLOAD_TOO_LARGE,
            WebErrorCode::RequestTooLarge,
            "请求内容超过大小限制。",
        )
    }

    pub fn internal() -> Self {
        Self::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            WebErrorCode::Internal,
            "服务暂时不可用，请稍后重试。",
        )
    }

    pub fn api_route_not_found() -> Self {
        Self::new(
            StatusCode::NOT_FOUND,
            WebErrorCode::ApiRouteNotFound,
            "API 路径不存在。",
        )
    }

    fn new(status: StatusCode, code: WebErrorCode, message: &'static str) -> Self {
        Self {
            status,
            code,
            message,
            retry_after: None,
        }
    }
}

impl IntoResponse for WebError {
    fn into_response(self) -> Response {
        let body = ErrorEnvelope {
            error: ErrorBody {
                code: self.code,
                message: self.message,
                request_id: request_id::current(),
            },
        };
        let mut response = (self.status, Json(body)).into_response();
        if let Some(value) = self.retry_after {
            response.headers_mut().insert(header::RETRY_AFTER, value);
        }
        response
    }
}
