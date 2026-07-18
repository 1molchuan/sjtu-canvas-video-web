#![allow(dead_code)]

use std::path::PathBuf;

mod app;
mod upstream;

use axum::{
    Router,
    body::{Body, to_bytes},
    http::{HeaderMap, Method, Request, Response, StatusCode, header},
};
use serde_json::Value;
use tower::ServiceExt;

pub const PUBLIC_ORIGIN: &str = "https://canvas-video.example.test";
pub(super) const CONTENT_HOST: &str = "content.example.test";

#[derive(Clone)]
pub struct BrowserAuth {
    pub cookie: String,
    pub csrf: String,
}

pub struct PendingAuth {
    pub cookie: String,
    pub events_url: String,
}

pub struct RequestSpec<'a> {
    method: Method,
    path: &'a str,
    cookie: Option<&'a str>,
    headers: HeaderMap,
    include_origin: bool,
}

impl<'a> RequestSpec<'a> {
    pub fn get(path: &'a str) -> Self {
        Self::new(Method::GET, path)
    }

    pub fn post(path: &'a str) -> Self {
        Self::new(Method::POST, path)
    }

    pub fn head(path: &'a str) -> Self {
        Self::new(Method::HEAD, path)
    }

    pub fn cookie(mut self, cookie: &'a str) -> Self {
        self.cookie = Some(cookie);
        self
    }

    pub fn headers(mut self, headers: HeaderMap) -> Self {
        self.headers = headers;
        self
    }

    pub fn without_origin(mut self) -> Self {
        self.include_origin = false;
        self
    }

    fn new(method: Method, path: &'a str) -> Self {
        Self {
            method,
            path,
            cookie: None,
            headers: HeaderMap::new(),
            include_origin: true,
        }
    }
}

pub struct ReadyDownload {
    pub auth: BrowserAuth,
    pub download_url: String,
}

pub struct ReadyTrack {
    pub auth: BrowserAuth,
    pub ticket_path: String,
}

pub struct TestHarness {
    pub app: Router,
    pub state: server::state::AppState,
    pub captures: upstream::CaptureStore,
}

#[derive(Clone)]
pub struct HarnessOptions {
    pub direct_downloads: bool,
    pub max_global_downloads: usize,
    pub max_downloads_per_user: usize,
    pub ticket_ttl_seconds: u64,
    pub protocol_timeout_millis: u64,
    pub login_mode: LoginMode,
    pub gateway_mode: GatewayMode,
    pub invite_database_path: Option<PathBuf>,
}

#[derive(Clone, Copy, Default)]
pub enum LoginMode {
    #[default]
    Allowed,
    Rejected,
    IdentityUnavailable,
    DuplicateScanned,
}

#[derive(Clone, Copy, Default)]
pub enum GatewayMode {
    #[default]
    Success,
    CoursesFail,
    VideosFail,
    DetailFail,
    SubtitleMissing,
}

impl Default for HarnessOptions {
    fn default() -> Self {
        Self {
            direct_downloads: false,
            max_global_downloads: 4,
            max_downloads_per_user: 1,
            ticket_ttl_seconds: 60,
            protocol_timeout_millis: 2_000,
            login_mode: LoginMode::Allowed,
            gateway_mode: GatewayMode::Success,
            invite_database_path: None,
        }
    }
}

pub async fn harness(options: HarnessOptions) -> TestHarness {
    let upstream = upstream::spawn().await;
    let (app, state) = app::build(upstream.origin, upstream.address, options);
    TestHarness {
        app,
        state,
        captures: upstream.captures,
    }
}

pub async fn ready_download(app: &Router) -> ReadyDownload {
    let ready = ready_track(app).await;
    let ticket = post(app, &ready.ticket_path, &ready.auth).await;
    ReadyDownload {
        auth: ready.auth,
        download_url: string_at(&ticket, "/download_url"),
    }
}

pub async fn ready_track(app: &Router) -> ReadyTrack {
    let auth = login(app).await;
    let courses = get_json(app, "/api/courses", &auth.cookie).await;
    let course = string_at(&courses, "/courses/0/id");
    let videos_path = format!("/api/courses/{course}/videos");
    let videos = get_json(app, &videos_path, &auth.cookie).await;
    let video = string_at(&videos, "/videos/0/id");
    let detail_path = format!("{videos_path}/{video}");
    let detail = get_json(app, &detail_path, &auth.cookie).await;
    let track = string_at(&detail, "/video/tracks/0/id");
    ReadyTrack {
        auth,
        ticket_path: format!("{detail_path}/tracks/{track}/ticket"),
    }
}

pub async fn login(app: &Router) -> BrowserAuth {
    let pending = start_pending(app).await;
    read_events(app, &pending).await;
    claim_session(app, &pending).await
}

pub async fn start_pending(app: &Router) -> PendingAuth {
    let start = request(app, RequestSpec::post("/api/auth/qr/start")).await;
    let cookie = cookie_pair(&start, "__Host-sjtu_canvas_video_session_pending=");
    let value = response_json(start).await;
    PendingAuth {
        cookie,
        events_url: string_at(&value, "/events_url"),
    }
}

pub async fn read_events(app: &Router, pending: &PendingAuth) -> String {
    let response = request(
        app,
        RequestSpec::get(&pending.events_url).cookie(&pending.cookie),
    )
    .await;
    let body = to_bytes(response.into_body(), 64 * 1024)
        .await
        .expect("terminal SSE");
    String::from_utf8(body.to_vec()).expect("UTF-8 SSE")
}

pub async fn claim_session(app: &Router, pending: &PendingAuth) -> BrowserAuth {
    let session = request(
        app,
        RequestSpec::get("/api/auth/session").cookie(&pending.cookie),
    )
    .await;
    let cookie = cookie_pair(&session, "__Host-sjtu_canvas_video_session=");
    let value = response_json(session).await;
    BrowserAuth {
        cookie,
        csrf: string_at(&value, "/csrf_token"),
    }
}

pub async fn request(app: &Router, spec: RequestSpec<'_>) -> Response<Body> {
    let mut builder = Request::builder().method(&spec.method).uri(spec.path);
    if spec.method == Method::POST && spec.include_origin {
        builder = builder.header(header::ORIGIN, PUBLIC_ORIGIN);
    }
    if let Some(value) = spec.cookie {
        builder = builder.header(header::COOKIE, value);
    }
    let mut request = builder.body(Body::empty()).expect("request");
    request.headers_mut().extend(spec.headers);
    app.clone().oneshot(request).await.expect("route")
}

pub async fn range_request(
    harness: &TestHarness,
    ready: &ReadyDownload,
    range: &str,
) -> Response<Body> {
    let mut headers = HeaderMap::new();
    headers.insert(header::RANGE, range.parse().expect("range"));
    request(
        &harness.app,
        RequestSpec::get(&ready.download_url)
            .cookie(&ready.auth.cookie)
            .headers(headers),
    )
    .await
}

pub async fn get_json(app: &Router, path: &str, cookie: &str) -> Value {
    let response = request(app, RequestSpec::get(path).cookie(cookie)).await;
    assert_eq!(response.status(), StatusCode::OK);
    response_json(response).await
}

pub async fn post(app: &Router, path: &str, auth: &BrowserAuth) -> Value {
    let mut headers = HeaderMap::new();
    headers.insert("X-CSRF-Token", auth.csrf.parse().expect("CSRF header"));
    let response = request(
        app,
        RequestSpec::post(path)
            .cookie(&auth.cookie)
            .headers(headers),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    response_json(response).await
}

pub async fn response_json(response: Response<Body>) -> Value {
    let body = to_bytes(response.into_body(), 64 * 1024)
        .await
        .expect("JSON body");
    serde_json::from_slice(&body).expect("JSON response")
}

pub fn cookie_pair(response: &Response<Body>, prefix: &str) -> String {
    response
        .headers()
        .get_all(header::SET_COOKIE)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .find(|value| value.starts_with(prefix))
        .and_then(|value| value.split(';').next())
        .expect("expected cookie")
        .to_owned()
}

pub fn string_at(value: &Value, pointer: &str) -> String {
    value
        .pointer(pointer)
        .and_then(Value::as_str)
        .expect("string field")
        .to_owned()
}
