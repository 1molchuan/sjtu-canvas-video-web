mod support;

use axum::http::{StatusCode, header};

use support::{HarnessOptions, RequestSpec, harness, login, ready_download, request};

#[tokio::test]
async fn explicit_direct_mode_redirects_without_contacting_upstream() {
    let harness = harness(HarnessOptions {
        direct_downloads: true,
        ..HarnessOptions::default()
    })
    .await;
    let ready = ready_download(&harness.app).await;

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
    assert!(harness.captures.lock().await.is_empty());
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
