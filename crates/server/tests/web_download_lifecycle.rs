mod support;

use axum::{
    body::to_bytes,
    http::{HeaderMap, StatusCode, header},
};

use support::{
    HarnessOptions, RequestSpec, harness, login, range_request, ready_download, request,
};

#[tokio::test]
async fn ticket_expires_and_logout_removes_it() {
    let harness = harness(HarnessOptions {
        ticket_ttl_seconds: 1,
        ..HarnessOptions::default()
    })
    .await;
    let expired = ready_download(&harness.app).await;
    tokio::time::sleep(std::time::Duration::from_millis(1_100)).await;
    let response = request(
        &harness.app,
        RequestSpec::get(&expired.download_url).cookie(&expired.auth.cookie),
    )
    .await;
    assert_eq!(response.status(), StatusCode::GONE);

    let active = ready_download(&harness.app).await;
    let mut headers = HeaderMap::new();
    headers.insert(
        "X-CSRF-Token",
        active.auth.csrf.parse().expect("CSRF header"),
    );
    let logout = request(
        &harness.app,
        RequestSpec::post("/api/auth/logout")
            .cookie(&active.auth.cookie)
            .headers(headers),
    )
    .await;
    assert_eq!(logout.status(), StatusCode::NO_CONTENT);
    let relogged = login(&harness.app).await;
    let removed = request(
        &harness.app,
        RequestSpec::get(&active.download_url).cookie(&relogged.cookie),
    )
    .await;
    assert_eq!(removed.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn per_user_limit_is_released_when_client_drops_body() {
    let harness = harness(HarnessOptions::default()).await;
    let ready = ready_download(&harness.app).await;
    let first = request(
        &harness.app,
        RequestSpec::get(&ready.download_url).cookie(&ready.auth.cookie),
    )
    .await;
    let blocked = request(
        &harness.app,
        RequestSpec::get(&ready.download_url).cookie(&ready.auth.cookie),
    )
    .await;
    assert_eq!(blocked.status(), StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(blocked.headers()[header::RETRY_AFTER], "5");
    drop(first);
    let released = request(
        &harness.app,
        RequestSpec::get(&ready.download_url).cookie(&ready.auth.cookie),
    )
    .await;
    assert_eq!(released.status(), StatusCode::OK);
}

#[tokio::test]
async fn global_limit_and_upstream_errors_release_permits() {
    let harness = harness(HarnessOptions {
        max_global_downloads: 1,
        max_downloads_per_user: 1,
        ..HarnessOptions::default()
    })
    .await;
    let first = ready_download(&harness.app).await;
    let second = ready_download(&harness.app).await;
    let held = request(
        &harness.app,
        RequestSpec::get(&first.download_url).cookie(&first.auth.cookie),
    )
    .await;
    let blocked = request(
        &harness.app,
        RequestSpec::get(&second.download_url).cookie(&second.auth.cookie),
    )
    .await;
    assert_eq!(blocked.status(), StatusCode::TOO_MANY_REQUESTS);
    drop(held);

    let upstream_error = range_request(&harness, &second, "bytes=500-").await;
    assert_eq!(upstream_error.status(), StatusCode::BAD_GATEWAY);
    let released = request(
        &harness.app,
        RequestSpec::get(&second.download_url).cookie(&second.auth.cookie),
    )
    .await;
    assert_eq!(released.status(), StatusCode::OK);
}

#[tokio::test]
async fn redirect_to_non_allowlisted_host_is_rejected() {
    let harness = harness(HarnessOptions::default()).await;
    let ready = ready_download(&harness.app).await;
    let response = range_request(&harness, &ready, "bytes=301-").await;
    assert_eq!(response.status(), StatusCode::BAD_GATEWAY);
    assert_eq!(harness.captures.lock().await.len(), 1);
}

#[tokio::test]
async fn shutdown_revokes_session_and_cancels_active_stream() {
    let harness = harness(HarnessOptions::default()).await;
    let ready = ready_download(&harness.app).await;
    let held = request(
        &harness.app,
        RequestSpec::get(&ready.download_url).cookie(&ready.auth.cookie),
    )
    .await;
    harness.state.begin_shutdown();
    assert!(
        to_bytes(held.into_body(), 32)
            .await
            .expect("cancelled body")
            .is_empty()
    );
    let rejected = request(
        &harness.app,
        RequestSpec::get(&ready.download_url).cookie(&ready.auth.cookie),
    )
    .await;
    assert_eq!(rejected.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn download_client_is_stateless_and_has_no_total_body_timeout() {
    let harness = harness(HarnessOptions {
        protocol_timeout_millis: 50,
        ..HarnessOptions::default()
    })
    .await;
    let ready = ready_download(&harness.app).await;
    let response = tokio::time::timeout(
        std::time::Duration::from_millis(100),
        range_request(&harness, &ready, "bytes=700-"),
    )
    .await
    .expect("proxy returns after upstream headers");
    assert_eq!(response.status(), StatusCode::PARTIAL_CONTENT);
    assert_eq!(
        to_bytes(response.into_body(), 8)
            .await
            .expect("delayed chunk"),
        "s"
    );

    let second = range_request(&harness, &ready, "bytes=0-0").await;
    assert_eq!(second.status(), StatusCode::PARTIAL_CONTENT);
    drop(second);
    for capture in harness.captures.lock().await.iter() {
        assert!(capture.cookie.is_none());
        assert!(capture.authorization.is_none());
    }
}

#[tokio::test]
async fn upstream_body_failure_releases_download_permits() {
    let harness = harness(HarnessOptions::default()).await;
    let ready = ready_download(&harness.app).await;
    let interrupted = range_request(&harness, &ready, "bytes=800-").await;
    assert_eq!(interrupted.status(), StatusCode::PARTIAL_CONTENT);
    assert!(to_bytes(interrupted.into_body(), 8).await.is_err());
    let retried = range_request(&harness, &ready, "bytes=0-0").await;
    assert_eq!(retried.status(), StatusCode::PARTIAL_CONTENT);
}

#[tokio::test]
async fn cleanup_removes_expired_sessions_and_tickets() {
    let harness = harness(HarnessOptions::default()).await;
    let ready = ready_download(&harness.app).await;
    let summary = harness
        .state
        .cleanup_expired(time::OffsetDateTime::now_utc() + time::Duration::hours(9));
    assert_eq!(summary.sessions, 1);
    assert_eq!(summary.tickets, 1);

    let rejected = request(
        &harness.app,
        RequestSpec::get(&ready.download_url).cookie(&ready.auth.cookie),
    )
    .await;
    assert_eq!(rejected.status(), StatusCode::UNAUTHORIZED);
}
