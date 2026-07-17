use std::time::Instant;

use axum::{
    extract::{MatchedPath, Request},
    middleware::Next,
    response::Response,
};

use super::request_id;

const UNKNOWN_ROUTE: &str = "<unmatched>";

pub async fn apply(request: Request, next: Next) -> Response {
    let started = Instant::now();
    let method = request.method().clone();
    let route = request
        .extensions()
        .get::<MatchedPath>()
        .map(MatchedPath::as_str)
        .unwrap_or(UNKNOWN_ROUTE)
        .to_owned();
    let response = next.run(request).await;
    let duration_ms = started.elapsed().as_millis() as u64;
    tracing::info!(
        request_id = %request_id::current(),
        method = %method,
        route,
        status = response.status().as_u16(),
        duration_ms,
        "request completed"
    );
    response
}
