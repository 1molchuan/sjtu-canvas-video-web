#![forbid(unsafe_code)]

pub mod auth;
pub mod config;
pub mod error;
pub mod gate;
pub mod gateway;
mod id;
pub mod middleware;
mod routes;
pub mod session;
pub mod shutdown;
pub mod state;
pub mod stream;
pub mod ticket;

use axum::Router;
use state::AppState;

pub fn app_router(state: AppState) -> Router {
    routes::router(state)
}

#[cfg(test)]
mod tests {
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use tower::ServiceExt;

    use super::routes;

    #[tokio::test]
    async fn health_endpoint_is_available() {
        let response = routes::health_router()
            .oneshot(
                Request::builder()
                    .uri("/api/health")
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("health route should respond");

        assert_eq!(response.status(), StatusCode::OK);
    }
}
