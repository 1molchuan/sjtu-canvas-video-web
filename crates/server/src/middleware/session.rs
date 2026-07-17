use std::sync::Arc;

use axum::{extract::FromRequestParts, http::request::Parts};
use time::OffsetDateTime;

use crate::{
    auth::browser_cookie, error::WebError, session::SessionLookupError, session::UserSession,
    state::AppState,
};

pub struct AuthenticatedSession(pub Arc<UserSession>);

impl FromRequestParts<AppState> for AuthenticatedSession {
    type Rejection = WebError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let id = browser_cookie::read_session(&parts.headers, state.config())
            .ok_or_else(WebError::unauthorized)?;
        let session = state
            .sessions()
            .lookup(&id, OffsetDateTime::now_utc())
            .map_err(map_lookup_error)?;
        Ok(Self(session))
    }
}

fn map_lookup_error(error: SessionLookupError) -> WebError {
    match error {
        SessionLookupError::Expired => WebError::session_expired(),
        SessionLookupError::Invalid | SessionLookupError::Revoked => WebError::unauthorized(),
    }
}
