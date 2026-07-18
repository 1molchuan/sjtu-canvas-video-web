use std::{sync::Arc, time::Instant};

use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    http::{HeaderMap, HeaderValue, header},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

use crate::{
    auth::{browser_cookie, invite, pending::PendingLoginId, sse, workflow},
    error::WebError,
    middleware::{cookies, csrf, origin, peer::PeerAddress},
    session::{SessionLookupError, UserSession, UserSessionOptions},
    state::AppState,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/auth/qr/start", post(start_qr))
        .route("/api/auth/qr/events/{pending_id}", get(qr_events))
        .route("/api/auth/session", get(auth_session))
        .route("/api/auth/logout", post(logout))
}

#[derive(Serialize)]
struct StartResponse {
    pending_id: String,
    events_url: String,
    expires_in_seconds: u64,
}

#[derive(Default, Deserialize)]
struct StartRequest {
    invite_token: Option<String>,
}

async fn start_qr(
    State(state): State<AppState>,
    PeerAddress(address): PeerAddress,
    headers: HeaderMap,
    request: Option<Json<StartRequest>>,
) -> Result<Response, WebError> {
    origin::verify(&headers, state.public_origin()).map_err(|_| WebError::origin_rejected())?;
    if !state.login_rate_limiter().allow(address, Instant::now()) {
        return Err(WebError::qr_start_rate_limited());
    }
    let now = OffsetDateTime::now_utc();
    let ttl = time::Duration::minutes(state.config().server.pending_login_ttl_minutes as i64);
    let invite = invite::reserve(
        &state,
        request.and_then(|Json(body)| body.invite_token),
        now,
        ttl,
    )
    .await?;
    let pending = match state
        .pending_logins()
        .create_with_invite(now, ttl, invite.clone())
    {
        Ok(pending) => pending,
        Err(_) => {
            invite::release(&state, invite).await?;
            return Err(WebError::too_many_pending());
        }
    };
    let body = StartResponse {
        pending_id: pending.id().expose().to_owned(),
        events_url: format!("/api/auth/qr/events/{}", pending.id().expose()),
        expires_in_seconds: state.config().server.pending_login_ttl_minutes * 60,
    };
    let cookie = browser_cookie::set_pending_cookie(
        state.config(),
        &browser_cookie::encode_pending(&pending),
    );
    workflow::spawn(state, pending);
    response_with_cookie(Json(body).into_response(), cookie)
}

async fn qr_events(
    State(state): State<AppState>,
    Path(raw_id): Path<String>,
    headers: HeaderMap,
) -> Result<Response, WebError> {
    let id = PendingLoginId::parse(&raw_id).ok_or_else(WebError::pending_not_found)?;
    let cookie = browser_cookie::read_pending(&headers, state.config())
        .ok_or_else(WebError::pending_not_found)?;
    if cookie.id != id {
        return Err(WebError::pending_not_found());
    }
    let pending = state
        .pending_logins()
        .get_authorized(&id, &cookie.binding, OffsetDateTime::now_utc())
        .ok_or_else(WebError::pending_not_found)?;
    let mut response = sse::response(&pending).into_response();
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("private, no-store"),
    );
    Ok(response)
}

#[derive(Serialize)]
struct SessionResponse {
    authenticated: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    user: Option<SessionUser>,
    #[serde(skip_serializing_if = "Option::is_none")]
    csrf_token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    expires_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    download_delivery: Option<&'static str>,
}

#[derive(Serialize)]
struct SessionUser {
    display_label: &'static str,
    identity_source: &'static str,
}

async fn auth_session(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Response, WebError> {
    let now = OffsetDateTime::now_utc();
    if let Some(session) = active_session(&state, &headers, now)? {
        return session_response(&session, state.config(), false);
    }
    let Some(pending_cookie) = browser_cookie::read_pending(&headers, state.config()) else {
        return Ok(Json(unauthenticated()).into_response());
    };
    let Some(login) = state.pending_logins().claim_authenticated(
        &pending_cookie.id,
        &pending_cookie.binding,
        now,
    ) else {
        return Ok(Json(unauthenticated()).into_response());
    };
    let expires_at = now + time::Duration::hours(state.config().server.session_ttl_hours as i64);
    let session = Arc::new(
        UserSession::new(
            login.identity,
            login.context,
            UserSessionOptions {
                expires_at,
                max_downloads: state.config().server.max_downloads_per_user,
            },
        )
        .map_err(|_| WebError::internal())?,
    );
    state.sessions().insert(session.clone());
    session_response(&session, state.config(), true)
}

async fn logout(State(state): State<AppState>, headers: HeaderMap) -> Result<Response, WebError> {
    origin::verify(&headers, state.public_origin()).map_err(|_| WebError::origin_rejected())?;
    let Some(id) = browser_cookie::read_session(&headers, state.config()) else {
        return cleared_logout_response(state.config());
    };
    let now = OffsetDateTime::now_utc();
    let Some(session) = state.sessions().get_active(&id, now) else {
        return cleared_logout_response(state.config());
    };
    let csrf_header = &state.config().security.csrf_header;
    let supplied = headers
        .get(csrf_header)
        .and_then(|value| value.to_str().ok())
        .ok_or_else(WebError::csrf_rejected)?;
    if !csrf::verify(&session, supplied) {
        return Err(WebError::csrf_rejected());
    }
    state.sessions().remove(&id);
    state.tickets().remove_for_session(&id);
    cleared_logout_response(state.config())
}

fn active_session(
    state: &AppState,
    headers: &HeaderMap,
    now: OffsetDateTime,
) -> Result<Option<Arc<UserSession>>, WebError> {
    let Some(id) = browser_cookie::read_session(headers, state.config()) else {
        return Ok(None);
    };
    match state.sessions().lookup(&id, now) {
        Ok(session) => Ok(Some(session)),
        Err(SessionLookupError::Expired) => Err(WebError::session_expired()),
        Err(SessionLookupError::Invalid | SessionLookupError::Revoked) => Ok(None),
    }
}

fn session_response(
    session: &Arc<UserSession>,
    config: &crate::config::AppConfig,
    set_cookie: bool,
) -> Result<Response, WebError> {
    let source = match session.identity().source {
        canvas_core::canvas::IdentitySource::MySjtuAccount => "my_sjtu",
        canvas_core::canvas::IdentitySource::CanvasSelf => "canvas",
    };
    let body = SessionResponse {
        authenticated: true,
        user: Some(SessionUser {
            display_label: "已登录用户",
            identity_source: source,
        }),
        csrf_token: Some(csrf::token(session)),
        expires_at: Some(
            session
                .expires_at()
                .format(&Rfc3339)
                .map_err(|_| WebError::internal())?,
        ),
        download_delivery: Some(config.server.download_delivery.browser_mode()),
    };
    let mut response = Json(body).into_response();
    if set_cookie {
        append_session_cookies(&mut response, session, config)?;
    }
    Ok(response)
}

fn append_session_cookies(
    response: &mut Response,
    session: &UserSession,
    config: &crate::config::AppConfig,
) -> Result<(), WebError> {
    let session_cookie = cookies::set_cookie(
        session.id().expose(),
        &browser_cookie::session_options(config),
    );
    append_set_cookie(response, session_cookie)?;
    append_set_cookie(response, browser_cookie::clear_pending_cookie(config))
}

fn response_with_cookie(mut response: Response, cookie: String) -> Result<Response, WebError> {
    append_set_cookie(&mut response, cookie)?;
    Ok(response)
}

fn append_set_cookie(response: &mut Response, cookie: String) -> Result<(), WebError> {
    let value = HeaderValue::from_str(&cookie).map_err(|_| WebError::internal())?;
    response.headers_mut().append(header::SET_COOKIE, value);
    Ok(())
}

fn cleared_logout_response(config: &crate::config::AppConfig) -> Result<Response, WebError> {
    let mut response = StatusCode::NO_CONTENT.into_response();
    append_set_cookie(
        &mut response,
        cookies::clear_cookie(&browser_cookie::session_options(config)),
    )?;
    append_set_cookie(&mut response, browser_cookie::clear_pending_cookie(config))?;
    Ok(response)
}

fn unauthenticated() -> SessionResponse {
    SessionResponse {
        authenticated: false,
        user: None,
        csrf_token: None,
        expires_at: None,
        download_delivery: None,
    }
}
