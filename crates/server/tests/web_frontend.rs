mod support;

use std::{fs, path::PathBuf};

use axum::{
    body::to_bytes,
    http::{HeaderMap, StatusCode, header},
};
use server::{FrontendAssets, app_router_with_frontend};
use support::{HarnessOptions, RequestSpec, harness, request};
use uuid::Uuid;

const INDEX_MARKER: &str = "phase-three-index";
const ASSET_BODY: &str = "console.log('hashed asset');";

struct FixtureDist {
    root: PathBuf,
}

impl FixtureDist {
    fn create() -> Self {
        let root = std::env::temp_dir().join(format!("canvas-video-frontend-{}", Uuid::new_v4()));
        fs::create_dir_all(root.join("assets")).expect("fixture assets directory");
        fs::write(
            root.join("index.html"),
            format!("<!doctype html><main>{INDEX_MARKER}</main>"),
        )
        .expect("fixture index");
        fs::write(root.join("assets/app-a1b2c3.js"), ASSET_BODY).expect("fixture asset");
        fs::write(
            root.join("favicon.svg"),
            "<svg xmlns='http://www.w3.org/2000/svg'/>",
        )
        .expect("fixture favicon");
        Self { root }
    }
}

impl Drop for FixtureDist {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

async fn production_app() -> (axum::Router, FixtureDist) {
    let fixture = FixtureDist::create();
    let assets = FrontendAssets::load(&fixture.root).expect("valid frontend fixture");
    let test = harness(HarnessOptions::default()).await;
    (app_router_with_frontend(test.state, assets), fixture)
}

#[tokio::test]
async fn serves_index_and_html_spa_routes_without_caching() {
    let (app, _fixture) = production_app().await;
    for path in ["/", "/courses/course-handle/videos/video-handle"] {
        let response = request_with_accept(&app, path, "text/html").await;
        assert_eq!(response.status(), StatusCode::OK);
        assert_header(response.headers(), header::CACHE_CONTROL, "no-cache");
        assert_header(
            response.headers(),
            header::CONTENT_TYPE,
            "text/html; charset=utf-8",
        );
        assert_header(response.headers(), "x-frame-options", "DENY");
        assert!(body_text(response).await.contains(INDEX_MARKER));
    }
}

#[tokio::test]
async fn serves_hashed_assets_with_immutable_cache_policy() {
    let (app, _fixture) = production_app().await;
    let response = request(&app, RequestSpec::get("/assets/app-a1b2c3.js")).await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_header(
        response.headers(),
        header::CACHE_CONTROL,
        "public, max-age=31536000, immutable",
    );
    assert_eq!(body_text(response).await, ASSET_BODY);
}

#[tokio::test]
async fn assets_and_non_html_requests_never_fall_back_to_index() {
    let (app, _fixture) = production_app().await;
    for (path, accept) in [
        ("/assets/missing.js", "text/html"),
        ("/courses/course-handle", "application/json"),
        ("/assets/%2e%2e/index.html", "text/html"),
    ] {
        let response = request_with_accept(&app, path, accept).await;
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        assert!(!body_text(response).await.contains(INDEX_MARKER));
    }
}

#[tokio::test]
async fn api_routes_never_fall_back_and_retain_api_cache_policy() {
    let (app, _fixture) = production_app().await;
    let missing = request_with_accept(&app, "/api/not-a-route", "text/html").await;
    assert_eq!(missing.status(), StatusCode::NOT_FOUND);
    assert_header(missing.headers(), header::CACHE_CONTROL, "no-store");
    let missing_body = body_text(missing).await;
    assert!(missing_body.contains("API_ROUTE_NOT_FOUND"));
    assert!(missing_body.contains("request_id"));

    let health = request(&app, RequestSpec::get("/api/health")).await;
    assert_eq!(health.status(), StatusCode::OK);
    assert_header(health.headers(), header::CACHE_CONTROL, "no-store");
    assert!(body_text(health).await.contains("\"status\":\"ok\""));
}

#[test]
fn frontend_assets_require_an_index_file() {
    let fixture = FixtureDist::create();
    fs::remove_file(fixture.root.join("index.html")).expect("remove fixture index");
    assert!(FrontendAssets::load(&fixture.root).is_err());
}

async fn request_with_accept(
    app: &axum::Router,
    path: &str,
    accept: &str,
) -> axum::response::Response {
    let mut headers = HeaderMap::new();
    headers.insert(header::ACCEPT, accept.parse().expect("valid accept header"));
    request(app, RequestSpec::get(path).headers(headers)).await
}

async fn body_text(response: axum::response::Response) -> String {
    let body = to_bytes(response.into_body(), 64 * 1024)
        .await
        .expect("response body");
    String::from_utf8(body.to_vec()).expect("UTF-8 response")
}

fn assert_header(headers: &HeaderMap, name: impl axum::http::header::AsHeaderName, value: &str) {
    assert_eq!(
        headers.get(name).and_then(|item| item.to_str().ok()),
        Some(value)
    );
}
