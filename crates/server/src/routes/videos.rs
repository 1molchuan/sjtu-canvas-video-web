use std::sync::Arc;

use axum::{
    Json, Router,
    extract::{Path, State},
    routing::get,
};
use serde::Serialize;
use time::OffsetDateTime;

use crate::{
    error::WebError,
    gateway::VideoDetailRequest,
    middleware::session::AuthenticatedSession,
    session::{
        CourseHandle, HandleWindow, TrackParent, TrackRegistration, TrackView, VideoHandle,
        VideoView,
    },
    state::AppState,
    stream::sanitize_filename,
};

const HANDLE_TTL_MINUTES: i64 = 5;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/courses/{course}/videos", get(list_videos))
        .route("/api/courses/{course}/videos/{video}", get(video_detail))
}

#[derive(Serialize)]
struct VideosResponse {
    videos: Vec<VideoResponse>,
}

#[derive(Serialize)]
struct VideoResponse {
    id: String,
    name: String,
    started_at: Option<String>,
}

#[derive(Serialize)]
struct DetailResponse {
    video: DetailVideo,
}

#[derive(Serialize)]
struct DetailVideo {
    id: String,
    name: String,
    tracks: Vec<TrackResponse>,
}

#[derive(Serialize)]
struct TrackResponse {
    id: String,
    kind: &'static str,
    suggested_filename: String,
}

pub(crate) struct DetailTarget {
    pub course: CourseHandle,
    pub video: VideoHandle,
    pub canvas_course_id: i64,
    pub real_video_id: String,
}

async fn list_videos(
    State(state): State<AppState>,
    AuthenticatedSession(session): AuthenticatedSession,
    Path(raw_course): Path<String>,
) -> Result<Json<VideosResponse>, WebError> {
    let _permit = session
        .protocol_permit()
        .await
        .ok_or_else(WebError::unauthorized)?;
    let course = parse_course(&raw_course)?;
    let now = OffsetDateTime::now_utc();
    let record = session
        .resources()
        .resolve_course(&course, now)
        .await
        .ok_or_else(WebError::invalid_course_handle)?;
    let (auth, videos) = state
        .protocol_gateway()
        .videos(session.protocol(), record.canvas_id)
        .await
        .map_err(|_| WebError::upstream_unavailable())?;
    ensure_active(&session)?;
    session
        .resources()
        .set_course_auth(course.clone(), Arc::new(auth))
        .await;
    let views = session
        .resources()
        .replace_videos(&course, videos, handle_window(now))
        .await
        .map_err(|_| WebError::invalid_course_handle())?;
    Ok(Json(VideosResponse {
        videos: views.into_iter().map(video_response).collect(),
    }))
}

async fn video_detail(
    State(state): State<AppState>,
    AuthenticatedSession(session): AuthenticatedSession,
    Path((raw_course, raw_video)): Path<(String, String)>,
) -> Result<Json<DetailResponse>, WebError> {
    let _permit = session
        .protocol_permit()
        .await
        .ok_or_else(WebError::unauthorized)?;
    let now = OffsetDateTime::now_utc();
    let target = resolve_detail_target(&session, (raw_course, raw_video), now).await?;
    let request = VideoDetailRequest {
        canvas_course_id: target.canvas_course_id,
        auth: session.resources().course_auth(&target.course).await,
        video_id: &target.real_video_id,
    };
    let detail = state
        .protocol_gateway()
        .video_detail(session.protocol(), request)
        .await
        .map_err(|_| WebError::upstream_unavailable())?;
    ensure_active(&session)?;
    session
        .resources()
        .set_course_auth(target.course.clone(), detail.auth)
        .await;
    let registrations = track_registrations(&session, detail.info.tracks).await?;
    let parent = TrackParent {
        course: target.course,
        video: target.video.clone(),
    };
    let tracks = session
        .resources()
        .replace_tracks(parent, registrations, handle_window(now))
        .await
        .map_err(|_| WebError::invalid_video_handle())?;
    Ok(Json(DetailResponse {
        video: DetailVideo {
            id: target.video.expose().to_owned(),
            name: detail.info.name,
            tracks: tracks.into_iter().map(track_response).collect(),
        },
    }))
}

pub(crate) async fn resolve_detail_target(
    session: &crate::session::UserSession,
    raw: (String, String),
    now: OffsetDateTime,
) -> Result<DetailTarget, WebError> {
    let course = parse_course(&raw.0)?;
    let video = VideoHandle::parse(&raw.1).ok_or_else(WebError::invalid_video_handle)?;
    let course_record = session
        .resources()
        .resolve_course(&course, now)
        .await
        .ok_or_else(WebError::invalid_course_handle)?;
    let video_record = session
        .resources()
        .resolve_video(&course, &video, now)
        .await
        .ok_or_else(WebError::invalid_video_handle)?;
    Ok(DetailTarget {
        course,
        video,
        canvas_course_id: course_record.canvas_id,
        real_video_id: video_record.real_id,
    })
}

async fn track_registrations(
    session: &Arc<crate::session::UserSession>,
    tracks: Vec<canvas_core::video::VideoTrack>,
) -> Result<Vec<TrackRegistration>, WebError> {
    let mut registrations = Vec::with_capacity(tracks.len());
    for track in tracks {
        registrations.push(
            TrackRegistration::from_track(session.protocol(), track)
                .await
                .map_err(|_| WebError::upstream_unavailable())?,
        );
    }
    Ok(registrations)
}

fn parse_course(raw: &str) -> Result<CourseHandle, WebError> {
    CourseHandle::parse(raw).ok_or_else(WebError::invalid_course_handle)
}

pub(crate) fn ensure_active(session: &Arc<crate::session::UserSession>) -> Result<(), WebError> {
    (!session.is_revoked())
        .then_some(())
        .ok_or_else(WebError::unauthorized)
}

fn handle_window(now: OffsetDateTime) -> HandleWindow {
    HandleWindow::new(now, time::Duration::minutes(HANDLE_TTL_MINUTES))
}

fn video_response(view: VideoView) -> VideoResponse {
    VideoResponse {
        id: view.handle.expose().to_owned(),
        name: view.name,
        started_at: view.started_at,
    }
}

fn track_response(view: TrackView) -> TrackResponse {
    let kind = match view.kind {
        canvas_core::video::VideoTrackKind::Camera => "camera",
        canvas_core::video::VideoTrackKind::Screen => "screen",
        canvas_core::video::VideoTrackKind::Mixed => "mixed",
        canvas_core::video::VideoTrackKind::Unknown => "unknown",
    };
    TrackResponse {
        id: view.handle.expose().to_owned(),
        kind,
        suggested_filename: sanitize_filename(&view.suggested_filename),
    }
}
