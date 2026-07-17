mod support;

use axum::http::{StatusCode, header};

use support::{
    HarnessOptions, PUBLIC_ORIGIN, RequestSpec, harness, login, ready_download, request,
    response_json,
};

#[tokio::test]
async fn explicit_direct_mode_probes_compatibility_then_redirects() {
    let harness = harness(HarnessOptions {
        direct_downloads: true,
        ..HarnessOptions::default()
    })
    .await;
    let ready = ready_download(&harness.app).await;
    let session = request(
        &harness.app,
        RequestSpec::get("/api/auth/session").cookie(&ready.auth.cookie),
    )
    .await;
    assert_eq!(
        response_json(session).await["download_delivery"],
        "direct_stream"
    );

    let response = request(
        &harness.app,
        RequestSpec::get(&ready.download_url).cookie(&ready.auth.cookie),
    )
    .await;

    assert_eq!(response.status(), StatusCode::TEMPORARY_REDIRECT);
    assert_eq!(
        response.headers()[header::CACHE_CONTROL],
        "private, no-store"
    );
    assert_eq!(response.headers()[header::REFERRER_POLICY], "no-referrer");
    assert!(response.headers().get(header::LOCATION).is_some());
    let captures = harness.captures.lock().await;
    assert_eq!(captures.len(), 3);
    assert_eq!(captures[0].range.as_deref(), Some("bytes=0-0"));
    assert!(captures[0].referer.is_none());
    assert!(captures[0].origin.is_none());
    assert_eq!(captures[1].range.as_deref(), Some("bytes=0-0"));
    assert!(captures[1].referer.is_none());
    assert_eq!(captures[1].origin.as_deref(), Some(PUBLIC_ORIGIN));
    assert_eq!(captures[2].range.as_deref(), Some("bytes=0-0"));
    assert_eq!(
        captures[2].referer.as_deref(),
        Some(canvas_core::constants::VIDEO_DOWNLOAD_REFERER)
    );
    assert!(captures[2].origin.is_none());
}

#[tokio::test]
async fn direct_mode_still_rejects_a_ticket_from_another_session() {
    let harness = harness(HarnessOptions {
        direct_downloads: true,
        ..HarnessOptions::default()
    })
    .await;
    let ready = ready_download(&harness.app).await;
    let other = login(&harness.app).await;

    let response = request(
        &harness.app,
        RequestSpec::get(&ready.download_url).cookie(&other.cookie),
    )
    .await;

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    assert!(response.headers().get(header::LOCATION).is_none());
}
