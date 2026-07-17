mod support;

use axum::{
    body::to_bytes,
    http::{StatusCode, header},
};
use serde_json::Value;

use support::{
    HarnessOptions, LoginMode, RequestSpec, claim_session, harness, read_events, request,
    start_pending,
};

#[tokio::test]
async fn non_whitelisted_identity_is_rejected_without_session() {
    let harness = harness(HarnessOptions {
        login_mode: LoginMode::Rejected,
        ..HarnessOptions::default()
    })
    .await;
    let pending = start_pending(&harness.app).await;
    let events = read_events(&harness.app, &pending).await;
    assert!(events.contains("\"type\":\"rejected\""));
    assert!(!events.contains("denied-user"));
    assert_unauthenticated(&harness.app, &pending.cookie).await;
}

#[tokio::test]
async fn missing_stable_identity_returns_safe_error_without_session() {
    let harness = harness(HarnessOptions {
        login_mode: LoginMode::IdentityUnavailable,
        ..HarnessOptions::default()
    })
    .await;
    let pending = start_pending(&harness.app).await;
    let events = read_events(&harness.app, &pending).await;
    assert!(events.contains("\"type\":\"error\""));
    assert!(events.contains("LOGIN_FAILED"));
    assert!(!events.contains("IdentityUnavailable"));
    assert_unauthenticated(&harness.app, &pending.cookie).await;
}

#[tokio::test]
async fn pending_id_and_browser_cookie_must_match() {
    let harness = harness(HarnessOptions::default()).await;
    let first = start_pending(&harness.app).await;
    let second = start_pending(&harness.app).await;
    let mismatch = request(
        &harness.app,
        RequestSpec::get(&first.events_url).cookie(&second.cookie),
    )
    .await;
    assert_eq!(mismatch.status(), StatusCode::NOT_FOUND);

    let malformed = request(
        &harness.app,
        RequestSpec::get("/api/auth/qr/events/not-an-opaque-id").cookie(&first.cookie),
    )
    .await;
    assert_eq!(malformed.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn terminal_sse_can_be_reconnected_before_single_session_claim() {
    let harness = harness(HarnessOptions::default()).await;
    let pending = start_pending(&harness.app).await;
    let first = read_events(&harness.app, &pending).await;
    let second = read_events(&harness.app, &pending).await;
    assert!(first.contains("\"type\":\"authenticated\""));
    assert!(second.contains("\"type\":\"authenticated\""));

    let session = claim_session(&harness.app, &pending).await;
    let already_claimed = request(
        &harness.app,
        RequestSpec::get("/api/auth/session").cookie(&pending.cookie),
    )
    .await;
    let value = response_json(already_claimed).await;
    assert_eq!(value["authenticated"], false);
    assert!(
        session
            .cookie
            .starts_with("__Host-sjtu_canvas_video_session=")
    );
}

#[tokio::test]
async fn duplicate_scanned_progress_is_published_once() {
    let harness = harness(HarnessOptions {
        login_mode: LoginMode::DuplicateScanned,
        ..HarnessOptions::default()
    })
    .await;
    let pending = start_pending(&harness.app).await;
    let events = read_events(&harness.app, &pending).await;
    assert_eq!(events.matches("\"type\":\"scanned\"").count(), 1);
    assert_eq!(events.matches("\"type\":\"authenticating\"").count(), 1);
    assert_eq!(events.matches("\"type\":\"authenticated\"").count(), 1);
}

async fn assert_unauthenticated(app: &axum::Router, pending_cookie: &str) {
    let response = request(
        app,
        RequestSpec::get("/api/auth/session").cookie(pending_cookie),
    )
    .await;
    assert!(
        response
            .headers()
            .get_all(header::SET_COOKIE)
            .iter()
            .filter_map(|value| value.to_str().ok())
            .all(|value| !value.starts_with("__Host-sjtu_canvas_video_session="))
    );
    let value = response_json(response).await;
    assert_eq!(value["authenticated"], false);
}

async fn response_json(response: axum::response::Response) -> Value {
    let body = to_bytes(response.into_body(), 16 * 1024)
        .await
        .expect("response body");
    serde_json::from_slice(&body).expect("response JSON")
}
