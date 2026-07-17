mod support;

use axum::{body::to_bytes, http::StatusCode};
use serde_json::Value;

use support::{HarnessOptions, RequestSpec, harness, login, request};

#[tokio::test]
async fn me_requires_session_and_exposes_only_minimal_identity() {
    let harness = harness(HarnessOptions::default()).await;
    let anonymous = request(&harness.app, RequestSpec::get("/api/me")).await;
    assert_eq!(anonymous.status(), StatusCode::UNAUTHORIZED);

    let auth = login(&harness.app).await;
    let response = request(
        &harness.app,
        RequestSpec::get("/api/me").cookie(&auth.cookie),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), 16 * 1024)
        .await
        .expect("me body");
    let value: Value = serde_json::from_slice(&body).expect("me JSON");
    assert_eq!(value["display_label"], "已登录用户");
    assert_eq!(value["identity_source"], "my_sjtu");
    assert!(value["expires_at"].as_str().is_some());
    let serialized = value.to_string();
    assert!(!serialized.contains("allowed-user"));
    assert!(!serialized.contains("stable_id"));
    assert!(!serialized.contains("csrf"));
}
