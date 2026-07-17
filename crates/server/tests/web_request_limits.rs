mod support;

use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode, header},
};
use support::{HarnessOptions, PUBLIC_ORIGIN, harness};
use tower::ServiceExt;

const TEST_BODY_BYTES: usize = 65_537;

#[tokio::test]
async fn oversized_body_is_rejected_before_a_pending_login_is_created() {
    let harness = harness(HarnessOptions::default()).await;
    let request = Request::builder()
        .method("POST")
        .uri("/api/auth/qr/start")
        .header(header::ORIGIN, PUBLIC_ORIGIN)
        .header(header::CONTENT_LENGTH, TEST_BODY_BYTES)
        .body(Body::from(vec![0_u8; TEST_BODY_BYTES]))
        .expect("request");
    let response = harness.app.clone().oneshot(request).await.expect("route");

    assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
    let body = to_bytes(response.into_body(), 16 * 1024)
        .await
        .expect("error body");
    let value: serde_json::Value = serde_json::from_slice(&body).expect("JSON error");
    assert_eq!(value["error"]["code"], "REQUEST_TOO_LARGE");
    assert!(value["error"]["request_id"].as_str().is_some());
}
