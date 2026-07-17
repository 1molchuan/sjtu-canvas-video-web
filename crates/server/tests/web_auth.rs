mod support;

use axum::{
    body::to_bytes,
    http::{HeaderMap, StatusCode, header},
};
use support::{
    HarnessOptions, RequestSpec, cookie_pair, harness, login, request, response_json, string_at,
};

const SESSION_COOKIE_PREFIX: &str = "__Host-sjtu_canvas_video_session=";
const PENDING_COOKIE_PREFIX: &str = "__Host-sjtu_canvas_video_session_pending=";

#[tokio::test]
async fn qr_login_uses_pending_cookie_then_rotates_to_session_cookie() {
    let harness = harness(HarnessOptions::default()).await;
    let start = request(&harness.app, RequestSpec::post("/api/auth/qr/start")).await;
    assert_eq!(start.status(), StatusCode::OK);
    let pending_cookie = cookie_pair(&start, PENDING_COOKIE_PREFIX);
    let start_json = response_json(start).await;
    let events_url = string_at(&start_json, "/events_url");

    let events = request(
        &harness.app,
        RequestSpec::get(&events_url).cookie(&pending_cookie),
    )
    .await;
    let text = String::from_utf8(
        to_bytes(events.into_body(), 64 * 1024)
            .await
            .expect("SSE body")
            .to_vec(),
    )
    .expect("UTF-8 SSE");
    assert_login_events(&text);

    let session = request(
        &harness.app,
        RequestSpec::get("/api/auth/session").cookie(&pending_cookie),
    )
    .await;
    assert_eq!(session.status(), StatusCode::OK);
    assert_session_cookie_is_secure(&session, &pending_cookie);
    assert_eq!(
        response_json(session).await["download_delivery"],
        "native_navigation"
    );
}

#[tokio::test]
async fn logout_requires_origin_and_session_bound_csrf_then_revokes_session() {
    let harness = harness(HarnessOptions::default()).await;
    let auth = login(&harness.app).await;
    let headers = csrf_headers(&auth.csrf);
    let rejected = request(
        &harness.app,
        RequestSpec::post("/api/auth/logout")
            .without_origin()
            .cookie(&auth.cookie)
            .headers(headers.clone()),
    )
    .await;
    assert_eq!(rejected.status(), StatusCode::FORBIDDEN);

    let logout = request(
        &harness.app,
        RequestSpec::post("/api/auth/logout")
            .cookie(&auth.cookie)
            .headers(headers),
    )
    .await;
    assert_eq!(logout.status(), StatusCode::NO_CONTENT);
    let after = request(
        &harness.app,
        RequestSpec::get("/api/auth/session").cookie(&auth.cookie),
    )
    .await;
    assert_eq!(response_json(after).await["authenticated"], false);
}

#[tokio::test]
async fn responses_have_security_headers_and_errors_have_request_id() {
    let harness = harness(HarnessOptions::default()).await;
    let mut headers = HeaderMap::new();
    headers.insert(
        header::ORIGIN,
        "https://attacker.example.test".parse().expect("origin"),
    );
    let response = request(
        &harness.app,
        RequestSpec::post("/api/auth/qr/start").headers(headers),
    )
    .await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    assert_security_headers(response.headers());
    let value = response_json(response).await;
    assert!(value["error"]["request_id"].as_str().is_some());
}

fn assert_login_events(text: &str) {
    for event_type in [
        "started",
        "qr",
        "scanned",
        "authenticating",
        "authenticated",
    ] {
        assert!(text.contains(&format!("\"type\":\"{event_type}\"")));
    }
}

fn assert_session_cookie_is_secure(response: &axum::response::Response, pending: &str) {
    let text = response
        .headers()
        .get_all(header::SET_COOKIE)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(text.contains(SESSION_COOKIE_PREFIX));
    assert!(text.contains("HttpOnly"));
    assert!(text.contains("Secure"));
    assert!(!text.contains(pending));
}

fn assert_security_headers(headers: &HeaderMap) {
    for name in [
        "content-security-policy",
        "x-content-type-options",
        "referrer-policy",
        "permissions-policy",
        "cross-origin-opener-policy",
        "x-frame-options",
        "x-request-id",
        "cache-control",
    ] {
        assert!(headers.contains_key(name), "missing {name}");
    }
    assert!(!headers.contains_key("access-control-allow-origin"));
}

fn csrf_headers(token: &str) -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert("X-CSRF-Token", token.parse().expect("CSRF"));
    headers
}
