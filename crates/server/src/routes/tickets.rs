use axum::{
    Json, Router,
    extract::{Path, State},
    http::HeaderMap,
    routing::post,
};
use serde::Serialize;
use time::OffsetDateTime;

use crate::{
    error::WebError,
    middleware::{csrf, origin, session::AuthenticatedSession},
    session::{CourseHandle, TrackHandle, TrackParent, VideoHandle},
    state::AppState,
    ticket::TicketRequest,
};

pub fn router() -> Router<AppState> {
    Router::new().route(
        "/api/courses/{course}/videos/{video}/tracks/{track}/ticket",
        post(issue_ticket),
    )
}

#[derive(Serialize)]
struct TicketResponse {
    download_url: String,
    expires_in_seconds: u64,
}

async fn issue_ticket(
    State(state): State<AppState>,
    (AuthenticatedSession(session), Path((raw_course, raw_video, raw_track)), headers): (
        AuthenticatedSession,
        Path<(String, String, String)>,
        HeaderMap,
    ),
) -> Result<Json<TicketResponse>, WebError> {
    verify_request(&state, &session, &headers)?;
    let course = CourseHandle::parse(&raw_course).ok_or_else(WebError::invalid_course_handle)?;
    let video = VideoHandle::parse(&raw_video).ok_or_else(WebError::invalid_video_handle)?;
    let track = TrackHandle::parse(&raw_track).ok_or_else(WebError::invalid_track_handle)?;
    let parent = TrackParent { course, video };
    let now = OffsetDateTime::now_utc();
    let record = session
        .resources()
        .resolve_track(&parent, &track, now)
        .await
        .ok_or_else(WebError::invalid_track_handle)?;
    let request = TicketRequest {
        session_id: session.id().clone(),
        parent,
        track,
        record,
    };
    let ttl_seconds = state.config().server.download_ticket_ttl_seconds;
    let ticket = state
        .tickets()
        .issue(request, now, time::Duration::seconds(ttl_seconds as i64))
        .map_err(|_| WebError::internal())?;
    Ok(Json(TicketResponse {
        download_url: format!("/api/download/{}", ticket.id().expose()),
        expires_in_seconds: ttl_seconds,
    }))
}

fn verify_request(
    state: &AppState,
    session: &crate::session::UserSession,
    headers: &HeaderMap,
) -> Result<(), WebError> {
    origin::verify(headers, state.public_origin()).map_err(|_| WebError::origin_rejected())?;
    let supplied = headers
        .get(&state.config().security.csrf_header)
        .and_then(|value| value.to_str().ok())
        .ok_or_else(WebError::csrf_rejected)?;
    csrf::verify(session, supplied)
        .then_some(())
        .ok_or_else(WebError::csrf_rejected)
}
