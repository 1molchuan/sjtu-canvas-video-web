use axum::{Json, Router, extract::State, routing::get};
use serde::Serialize;
use time::OffsetDateTime;

use crate::{
    error::WebError,
    middleware::session::AuthenticatedSession,
    session::{CourseView, HandleWindow},
    state::AppState,
};

const HANDLE_TTL_MINUTES: i64 = 5;

pub fn router() -> Router<AppState> {
    Router::new().route("/api/courses", get(list_courses))
}

#[derive(Serialize)]
struct CoursesResponse {
    courses: Vec<CourseResponse>,
}

#[derive(Serialize)]
struct CourseResponse {
    id: String,
    name: String,
    course_code: Option<String>,
    term_name: Option<String>,
}

async fn list_courses(
    State(state): State<AppState>,
    AuthenticatedSession(session): AuthenticatedSession,
) -> Result<Json<CoursesResponse>, WebError> {
    let _permit = session
        .protocol_permit()
        .await
        .ok_or_else(WebError::unauthorized)?;
    let courses = state
        .protocol_gateway()
        .courses(session.protocol())
        .await
        .map_err(|_| WebError::upstream_unavailable())?;
    if session.is_revoked() {
        return Err(WebError::unauthorized());
    }
    let now = OffsetDateTime::now_utc();
    let window = HandleWindow::new(now, time::Duration::minutes(HANDLE_TTL_MINUTES));
    let views = session
        .resources()
        .replace_courses(courses, window)
        .await
        .map_err(|_| WebError::internal())?;
    Ok(Json(CoursesResponse {
        courses: views.into_iter().map(course_response).collect(),
    }))
}

fn course_response(view: CourseView) -> CourseResponse {
    CourseResponse {
        id: view.handle.expose().to_owned(),
        name: view.name,
        course_code: view.course_code,
        term_name: view.term_name,
    }
}
