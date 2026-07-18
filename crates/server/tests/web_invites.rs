mod support;

use std::{fs, path::PathBuf};

use axum::{
    body::Body,
    http::{Request, StatusCode, header},
};
use secrecy::ExposeSecret;
use serde_json::{Value, json};
use server::invite::InviteStore;
use time::{Duration, OffsetDateTime};
use tower::ServiceExt;
use uuid::Uuid;

use support::{
    HarnessOptions, LoginMode, PUBLIC_ORIGIN, RequestSpec, claim_session, cookie_pair, harness,
    read_events, request, response_json,
};

struct TestDatabase(PathBuf);

impl TestDatabase {
    fn new() -> Self {
        Self(std::env::temp_dir().join(format!(
            "canvas-video-web-invite-{}.sqlite3",
            Uuid::new_v4()
        )))
    }
}

impl Drop for TestDatabase {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
        let _ = fs::remove_file(self.0.with_extension("sqlite3-shm"));
        let _ = fs::remove_file(self.0.with_extension("sqlite3-wal"));
    }
}

#[tokio::test]
async fn invite_enrolls_rejected_identity_and_is_single_use() {
    let database = TestDatabase::new();
    let store = InviteStore::open(&database.0).expect("invite store");
    let invitation = store
        .create(OffsetDateTime::now_utc(), Duration::hours(24))
        .expect("invitation");
    let app = harness(HarnessOptions {
        login_mode: LoginMode::Rejected,
        invite_database_path: Some(database.0.clone()),
        ..HarnessOptions::default()
    })
    .await
    .app;

    let pending = start_with_invite(&app, invitation.token().expose_secret()).await;
    let events = read_events(&app, &pending).await;
    assert!(events.contains("\"type\":\"authenticated\""));
    let session = claim_session(&app, &pending).await;
    assert!(
        session
            .cookie
            .starts_with("__Host-sjtu_canvas_video_session=")
    );

    let reused = start_response(&app, invitation.token().expose_secret()).await;
    assert_eq!(reused.status(), StatusCode::GONE);

    let normal = support::start_pending(&app).await;
    let events = read_events(&app, &normal).await;
    assert!(events.contains("\"type\":\"authenticated\""));
}

async fn start_with_invite(app: &axum::Router, token: &str) -> support::PendingAuth {
    let response = start_response(app, token).await;
    assert_eq!(response.status(), StatusCode::OK);
    let cookie = cookie_pair(&response, "__Host-sjtu_canvas_video_session_pending=");
    let value = response_json(response).await;
    support::PendingAuth {
        cookie,
        events_url: text(&value, "/events_url"),
    }
}

async fn start_response(app: &axum::Router, token: &str) -> axum::response::Response {
    let request = Request::builder()
        .method("POST")
        .uri("/api/auth/qr/start")
        .header(header::ORIGIN, PUBLIC_ORIGIN)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            serde_json::to_vec(&json!({ "invite_token": token })).expect("request JSON"),
        ))
        .expect("request");
    app.clone().oneshot(request).await.expect("route")
}

fn text(value: &Value, pointer: &str) -> String {
    value
        .pointer(pointer)
        .and_then(Value::as_str)
        .expect("string field")
        .to_owned()
}

#[tokio::test]
async fn malformed_invite_is_rejected_before_pending_login_is_created() {
    let database = TestDatabase::new();
    let harness = harness(HarnessOptions {
        invite_database_path: Some(database.0.clone()),
        ..HarnessOptions::default()
    })
    .await;

    let response = start_response(&harness.app, "not-a-valid-token").await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    assert_eq!(harness.state.pending_login_count(), 0);
    let cookies = response.headers().get_all(header::SET_COOKIE);
    assert_eq!(cookies.iter().count(), 0);

    let ordinary = request(&harness.app, RequestSpec::post("/api/auth/qr/start")).await;
    assert_eq!(ordinary.status(), StatusCode::OK);
}

#[tokio::test]
async fn invite_is_consumed_even_when_identity_was_already_configured() {
    let database = TestDatabase::new();
    let store = InviteStore::open(&database.0).expect("invite store");
    let invitation = store
        .create(OffsetDateTime::now_utc(), Duration::hours(24))
        .expect("invitation");
    let app = harness(HarnessOptions {
        invite_database_path: Some(database.0.clone()),
        ..HarnessOptions::default()
    })
    .await
    .app;

    let pending = start_with_invite(&app, invitation.token().expose_secret()).await;
    read_events(&app, &pending).await;
    claim_session(&app, &pending).await;

    let allowed = store.list_allowed().expect("dynamic allowlist");
    assert_eq!(allowed.len(), 1);
    assert_eq!(allowed[0].invite_id(), invitation.id());
}
