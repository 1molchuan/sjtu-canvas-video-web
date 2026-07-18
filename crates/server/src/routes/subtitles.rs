use axum::{
    Router,
    body::Body,
    extract::{Path, State},
    http::{Response, header},
    routing::get,
};
use canvas_core::ProtocolError;
use time::OffsetDateTime;

use crate::{
    error::WebError, gateway::VideoDetailRequest, middleware::session::AuthenticatedSession,
    state::AppState, stream::subtitle_attachment_content_disposition,
};

use super::videos::{ensure_active, resolve_detail_target};

const SUBTITLE_CONTENT_TYPE: &str = "application/x-subrip; charset=utf-8";

pub fn router() -> Router<AppState> {
    Router::new().route(
        "/api/courses/{course}/videos/{video}/subtitle",
        get(download_subtitle),
    )
}

async fn download_subtitle(
    State(state): State<AppState>,
    AuthenticatedSession(session): AuthenticatedSession,
    Path(raw): Path<(String, String)>,
) -> Result<Response<Body>, WebError> {
    let _permit = session
        .protocol_permit()
        .await
        .ok_or_else(WebError::unauthorized)?;
    let target = resolve_detail_target(&session, raw, OffsetDateTime::now_utc()).await?;
    let request = VideoDetailRequest {
        canvas_course_id: target.canvas_course_id,
        auth: session.resources().course_auth(&target.course).await,
        video_id: &target.real_video_id,
    };
    let subtitle = state
        .protocol_gateway()
        .subtitle(session.protocol(), request)
        .await
        .map_err(map_subtitle_error)?;
    ensure_active(&session)?;
    session
        .resources()
        .set_course_auth(target.course, subtitle.auth)
        .await;
    build_response(subtitle.video_name, subtitle.document.srt)
}

fn build_response(video_name: String, srt: String) -> Result<Response<Body>, WebError> {
    let disposition = subtitle_attachment_content_disposition(&format!("{video_name}.srt"))
        .map_err(|_| WebError::internal())?;
    Response::builder()
        .header(header::CONTENT_TYPE, SUBTITLE_CONTENT_TYPE)
        .header(header::CONTENT_DISPOSITION, disposition)
        .header(header::CACHE_CONTROL, "private, no-store")
        .body(Body::from(srt))
        .map_err(|_| WebError::internal())
}

fn map_subtitle_error(error: ProtocolError) -> WebError {
    match error {
        ProtocolError::SubtitleMissing => WebError::subtitle_missing(),
        _ => WebError::upstream_unavailable(),
    }
}
