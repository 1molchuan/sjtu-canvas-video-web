mod support;

use axum::{
    body::to_bytes,
    http::{HeaderMap, StatusCode, header},
};
use serde_json::Value;

use support::{HarnessOptions, PUBLIC_ORIGIN, RequestSpec, harness, login, ready_track, request};

#[tokio::test]
async fn ticket_requires_exact_origin_and_session_csrf() {
    let harness = harness(HarnessOptions::default()).await;
    let ready = ready_track(&harness.app).await;
    let missing = request(
        &harness.app,
        RequestSpec::post(&ready.ticket_path).cookie(&ready.auth.cookie),
    )
    .await;
    assert_eq!(missing.status(), StatusCode::FORBIDDEN);

    let other = login(&harness.app).await;
    let cross_session = issue_with_headers(
        &harness.app,
        TicketAttempt {
            path: &ready.ticket_path,
            cookie: &ready.auth.cookie,
            origin: PUBLIC_ORIGIN,
            csrf: &other.csrf,
        },
    )
    .await;
    assert_eq!(cross_session.status(), StatusCode::FORBIDDEN);

    let wrong_origin = issue_with_headers(
        &harness.app,
        TicketAttempt {
            path: &ready.ticket_path,
            cookie: &ready.auth.cookie,
            origin: "https://attacker.example.test",
            csrf: &ready.auth.csrf,
        },
    )
    .await;
    assert_eq!(wrong_origin.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn ticket_response_is_opaque_and_session_bound() {
    let harness = harness(HarnessOptions::default()).await;
    let ready = ready_track(&harness.app).await;
    let response = issue_with_headers(
        &harness.app,
        TicketAttempt {
            path: &ready.ticket_path,
            cookie: &ready.auth.cookie,
            origin: PUBLIC_ORIGIN,
            csrf: &ready.auth.csrf,
        },
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), 16 * 1024)
        .await
        .expect("ticket body");
    let value: Value = serde_json::from_slice(&body).expect("ticket JSON");
    let url = value["download_url"].as_str().expect("download URL");
    assert!(url.starts_with("/api/download/"));
    assert!(!url.contains("http"));
    assert!(!url.contains("credential"));
    assert_eq!(value["expires_in_seconds"], 60);
}

struct TicketAttempt<'a> {
    path: &'a str,
    cookie: &'a str,
    origin: &'a str,
    csrf: &'a str,
}

async fn issue_with_headers(
    app: &axum::Router,
    attempt: TicketAttempt<'_>,
) -> axum::response::Response {
    let mut headers = HeaderMap::new();
    headers.insert(header::ORIGIN, attempt.origin.parse().expect("origin"));
    headers.insert("X-CSRF-Token", attempt.csrf.parse().expect("CSRF"));
    request(
        app,
        RequestSpec::post(attempt.path)
            .cookie(attempt.cookie)
            .headers(headers),
    )
    .await
}
