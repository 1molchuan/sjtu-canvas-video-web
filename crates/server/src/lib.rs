#![forbid(unsafe_code)]

pub mod config;
mod routes;

use axum::Router;

pub fn app_router() -> Router {
    routes::router()
}

#[cfg(test)]
mod tests {
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use tower::ServiceExt;

    use super::app_router;

    #[tokio::test]
    async fn health_endpoint_is_available() {
        let response = app_router()
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
