mod support;

use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicUsize, Ordering},
};

use axum::{
    Router,
    extract::State,
    http::{HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
};
use canvas_core::{
    canvas::establish_canvas_session,
    client::{ProtocolContext, UpstreamPurpose},
    error::ProtocolError,
};
use secrecy::SecretString;
use tokio::sync::Mutex;

use support::{
    MockServer, Shared,
    topology::{MockTopology, redirect_url},
};

const JA_COOKIE: &str = "ja-cookie-secret-canary";

#[derive(Default)]
struct SsoObservations {
    malicious_redirect: AtomicBool,
    video_hits: AtomicUsize,
    canvas_login_cookie: Mutex<Option<String>>,
    jaccount_cookie: Mutex<Option<String>>,
    protected_cookie: Mutex<Option<String>>,
}

#[tokio::test]
async fn canvas_sso_copies_ja_cookie_only_to_identity_origins() {
    let observations = Arc::new(SsoObservations::default());
    let server = MockServer::spawn(router(observations.clone())).await;
    let topology = MockTopology::for_server(&server);
    let context = ProtocolContext::new(topology.config).expect("context should build");
    context
        .client
        .get(
            context
                .endpoints
                .jaccount_origin()
                .join("ja/wide-cookie")
                .expect("mock cookie path is valid"),
        )
        .send()
        .await
        .expect("wide upstream Cookie fixture should respond");

    let status = establish_canvas_session(&context, &SecretString::from(JA_COOKIE.to_owned()))
        .await
        .expect("mock Canvas SSO should succeed");

    assert!(status.authenticated);
    assert_eq!(status.final_host.as_deref(), Some(topology.canvas_host));
    assert!(status.cookie_names.contains(&"CanvasSession".to_owned()));
    assert_eq!(*observations.canvas_login_cookie.lock().await, None);
    assert!(
        observations
            .jaccount_cookie
            .lock()
            .await
            .as_deref()
            .is_some_and(|value| value.contains("JAAuthCookie="))
    );
    assert!(
        observations
            .protected_cookie
            .lock()
            .await
            .as_deref()
            .is_some_and(|value| value.contains("CanvasSession="))
    );
}

#[tokio::test]
async fn canvas_sso_rejects_video_origin_redirect_before_request() {
    let observations = Arc::new(SsoObservations::default());
    observations
        .malicious_redirect
        .store(true, Ordering::SeqCst);
    let server = MockServer::spawn(router(observations.clone())).await;
    let topology = MockTopology::for_server(&server);
    let context = ProtocolContext::new(topology.config).expect("context should build");

    let error = establish_canvas_session(&context, &SecretString::from(JA_COOKIE.to_owned()))
        .await
        .expect_err("cross-purpose redirect must fail");

    assert!(matches!(
        error,
        ProtocolError::InvalidUpstreamUrl {
            purpose: UpstreamPurpose::Canvas,
            ..
        }
    ));
    assert_eq!(observations.video_hits.load(Ordering::SeqCst), 0);
}

fn router(observations: Shared<SsoObservations>) -> Router {
    Router::new()
        .route("/canvas/login", get(canvas_login))
        .route("/ja/wide-cookie", get(wide_cookie))
        .route("/ja/sso", get(jaccount_sso))
        .route("/canvas/callback", get(canvas_callback))
        .route("/canvas/protected", get(canvas_protected))
        .route("/video/should-not-follow", get(video_target))
        .with_state(observations)
}

async fn wide_cookie() -> Response {
    let mut response = "ok".into_response();
    response.headers_mut().insert(
        "set-cookie",
        HeaderValue::from_static(
            "JAAuthCookie=ja-cookie-secret-canary; Domain=.sjtu.mock.test; Path=/; HttpOnly",
        ),
    );
    response
}

async fn canvas_login(
    State(state): State<Shared<SsoObservations>>,
    headers: HeaderMap,
) -> Response {
    *state.canvas_login_cookie.lock().await = cookie_header(&headers);
    let target = if state.malicious_redirect.load(Ordering::SeqCst) {
        redirect_url(
            &headers,
            "content.sjtu.mock.test",
            "/video/should-not-follow",
        )
    } else {
        redirect_url(&headers, "jaccount.sjtu.mock.test", "/ja/sso")
    };
    redirect(target)
}

async fn jaccount_sso(
    State(state): State<Shared<SsoObservations>>,
    headers: HeaderMap,
) -> Response {
    *state.jaccount_cookie.lock().await = cookie_header(&headers);
    redirect(redirect_url(
        &headers,
        "canvas.sjtu.mock.test",
        "/canvas/callback",
    ))
}

async fn canvas_callback(headers: HeaderMap) -> Response {
    let target = redirect_url(&headers, "canvas.sjtu.mock.test", "/canvas/protected");
    let mut response = redirect(target);
    response.headers_mut().insert(
        "set-cookie",
        HeaderValue::from_static("CanvasSession=canvas-secret; Path=/; HttpOnly"),
    );
    response
}

async fn canvas_protected(
    State(state): State<Shared<SsoObservations>>,
    headers: HeaderMap,
) -> &'static str {
    *state.protected_cookie.lock().await = cookie_header(&headers);
    "canvas-authenticated"
}

async fn video_target(State(state): State<Shared<SsoObservations>>) -> &'static str {
    state.video_hits.fetch_add(1, Ordering::SeqCst);
    "must not be reached"
}

fn redirect(location: String) -> Response {
    let mut response = StatusCode::FOUND.into_response();
    response.headers_mut().insert(
        "location",
        HeaderValue::from_str(&location).expect("mock location is valid"),
    );
    response
}

fn cookie_header(headers: &HeaderMap) -> Option<String> {
    headers
        .get("cookie")
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned)
}
