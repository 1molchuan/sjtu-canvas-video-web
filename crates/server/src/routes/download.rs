use std::sync::Arc;

use axum::{
    Router,
    extract::{Path, State},
    http::{HeaderMap, Method, Response, header},
    routing::get,
};
use time::OffsetDateTime;
use tokio::sync::Semaphore;

use crate::{
    config::DownloadDelivery,
    error::WebError,
    middleware::session::AuthenticatedSession,
    session::UserSession,
    state::AppState,
    stream::{
        ByteRange, DownloadPermits, ProxyOptions, RangeParseError, direct_download_redirect,
        parse_single_range, proxy_download,
    },
    ticket::{DownloadTicket, DownloadTicketId, TicketLookupError},
};

pub fn router() -> Router<AppState> {
    Router::new().route("/api/download/{ticket}", get(download).head(download))
}

async fn download(
    State(state): State<AppState>,
    (AuthenticatedSession(session), Path(raw_ticket), method, headers): (
        AuthenticatedSession,
        Path<String>,
        Method,
        HeaderMap,
    ),
) -> Result<Response<axum::body::Body>, WebError> {
    let range = requested_range(&headers)?;
    let ticket = resolve_ticket(&state, &session, &raw_ticket)?;
    verify_resource_binding(&session, &ticket).await?;
    if state.config().server.download_delivery == DownloadDelivery::RedirectExperimental {
        return direct_download_redirect(session.protocol(), ticket.resource()).await;
    }
    let permits = acquire_permits(
        state.global_download_semaphore(),
        session.download_semaphore(),
    )?;
    let options = ProxyOptions {
        method,
        range,
        resource: ticket.resource().clone(),
        suggested_filename: ticket.suggested_filename().to_owned(),
        permits,
        session_revoked: session.revocation(),
        shutting_down: state.shutdown(),
    };
    proxy_download(session.protocol(), options).await
}

fn requested_range(headers: &HeaderMap) -> Result<Option<ByteRange>, WebError> {
    let Some(value) = headers.get(header::RANGE) else {
        return Ok(None);
    };
    let value = value.to_str().map_err(|_| WebError::invalid_range())?;
    parse_single_range(value).map(Some).map_err(map_range_error)
}

fn map_range_error(error: RangeParseError) -> WebError {
    if error == RangeParseError::MultipleRanges {
        return WebError::multiple_ranges_unsupported();
    }
    WebError::invalid_range()
}

fn resolve_ticket(
    state: &AppState,
    session: &UserSession,
    raw_ticket: &str,
) -> Result<Arc<DownloadTicket>, WebError> {
    let id = DownloadTicketId::parse(raw_ticket).ok_or_else(WebError::invalid_download_ticket)?;
    state
        .tickets()
        .resolve(&id, session.id(), OffsetDateTime::now_utc())
        .map_err(map_ticket_error)
}

fn map_ticket_error(error: TicketLookupError) -> WebError {
    match error {
        TicketLookupError::Invalid => WebError::invalid_download_ticket(),
        TicketLookupError::Expired => WebError::expired_download_ticket(),
        TicketLookupError::SessionMismatch => WebError::download_ticket_session_mismatch(),
    }
}

async fn verify_resource_binding(
    session: &UserSession,
    ticket: &DownloadTicket,
) -> Result<(), WebError> {
    session
        .resources()
        .resolve_track(ticket.parent(), ticket.track(), OffsetDateTime::now_utc())
        .await
        .map(|_| ())
        .ok_or_else(WebError::invalid_track_handle)
}

fn acquire_permits(
    global: Arc<Semaphore>,
    user: Arc<Semaphore>,
) -> Result<DownloadPermits, WebError> {
    let global = global
        .try_acquire_owned()
        .map_err(|_| WebError::download_limit_exceeded())?;
    let user = user
        .try_acquire_owned()
        .map_err(|_| WebError::download_limit_exceeded())?;
    Ok(DownloadPermits::new(global, user))
}
