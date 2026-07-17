mod auth;
mod courses;
mod download;
mod health;
mod me;
mod tickets;
mod videos;

use axum::{Router, middleware};
use tower_http::limit::RequestBodyLimitLayer;

use crate::state::AppState;

pub fn router(state: AppState) -> Router {
    let body_limit = state.config().security.max_request_body_bytes;
    Router::new()
        .merge(health::router())
        .merge(me::router())
        .merge(auth::router())
        .merge(courses::router())
        .merge(download::router())
        .merge(videos::router())
        .merge(tickets::router())
        .with_state(state)
        .layer(RequestBodyLimitLayer::new(body_limit))
        .layer(middleware::from_fn(
            crate::middleware::request_limit::normalize_error,
        ))
        .layer(middleware::from_fn(
            crate::middleware::security_headers::apply,
        ))
        .layer(middleware::from_fn(crate::middleware::request_log::apply))
        .layer(middleware::from_fn(crate::middleware::request_id::apply))
}

#[cfg(test)]
pub(crate) fn health_router() -> Router {
    health::router()
}
