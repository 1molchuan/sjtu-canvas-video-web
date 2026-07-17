use axum::{Json, Router, routing::get};
use serde::Serialize;
use time::format_description::well_known::Rfc3339;

use crate::{error::WebError, middleware::session::AuthenticatedSession, state::AppState};

pub fn router() -> Router<AppState> {
    Router::new().route("/api/me", get(me))
}

#[derive(Serialize)]
struct MeResponse {
    display_label: &'static str,
    identity_source: &'static str,
    expires_at: String,
}

async fn me(
    AuthenticatedSession(session): AuthenticatedSession,
) -> Result<Json<MeResponse>, WebError> {
    let identity_source = match session.identity().source {
        canvas_core::canvas::IdentitySource::MySjtuAccount => "my_sjtu",
        canvas_core::canvas::IdentitySource::CanvasSelf => "canvas",
    };
    let expires_at = session
        .expires_at()
        .format(&Rfc3339)
        .map_err(|_| WebError::internal())?;
    Ok(Json(MeResponse {
        display_label: "已登录用户",
        identity_source,
        expires_at,
    }))
}
