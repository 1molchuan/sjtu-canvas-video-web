mod auth;
mod courses;
mod download;
mod health;
mod me;
mod subtitles;
mod tickets;
mod videos;

use axum::{Router, middleware, routing::any};
use tower_http::limit::RequestBodyLimitLayer;

use crate::{
    error::WebError,
    frontend::{self, FrontendAssets},
    state::AppState,
};

pub fn router(state: AppState) -> Router {
    finish(api_routes(), state)
}

pub fn router_with_frontend(state: AppState, assets: FrontendAssets) -> Router {
    finish(api_routes().merge(frontend::router(assets)), state)
}

fn api_routes() -> Router<AppState> {
    Router::new()
        .merge(health::router())
        .merge(me::router())
        .merge(auth::router())
        .merge(courses::router())
        .merge(download::router())
        .merge(videos::router())
        .merge(subtitles::router())
        .merge(tickets::router())
        .route("/api", any(api_not_found))
        .route("/api/{*path}", any(api_not_found))
}

fn finish(routes: Router<AppState>, state: AppState) -> Router {
    let body_limit = state.config().security.max_request_body_bytes;
    routes
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

async fn api_not_found() -> WebError {
    WebError::api_route_not_found()
}

#[cfg(test)]
pub(crate) fn health_router() -> Router {
    health::router()
}
