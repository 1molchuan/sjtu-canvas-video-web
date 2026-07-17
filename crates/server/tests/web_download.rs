mod support;

use axum::{
    body::to_bytes,
    http::{HeaderMap, Method, StatusCode, header},
};

use support::{
    HarnessOptions, RequestSpec, harness, login, range_request, ready_download, request,
};

#[tokio::test]
async fn full_get_and_head_forward_safe_metadata() {
    let harness = harness(HarnessOptions::default()).await;
    let ready = ready_download(&harness.app).await;
    let response = request(
        &harness.app,
        RequestSpec::get(&ready.download_url).cookie(&ready.auth.cookie),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()[header::CONTENT_LENGTH], "15");
    assert_eq!(
        to_bytes(response.into_body(), 32).await.expect("full body"),
        "synthetic-video"
    );

    let head = request(
        &harness.app,
        RequestSpec::head(&ready.download_url).cookie(&ready.auth.cookie),
    )
    .await;
    assert_eq!(head.status(), StatusCode::OK);
    assert_eq!(head.headers()[header::CONTENT_LENGTH], "15");
    assert!(
        to_bytes(head.into_body(), 1)
            .await
            .expect("HEAD body")
            .is_empty()
    );
}

#[tokio::test]
async fn range_download_is_streamed_with_filtered_headers() {
    let harness = harness(HarnessOptions::default()).await;
    let ready = ready_download(&harness.app).await;
    let mut headers = HeaderMap::new();
    headers.insert(header::RANGE, "bytes=0-0".parse().expect("range"));
    let response = request(
        &harness.app,
        RequestSpec::get(&ready.download_url)
            .cookie(&ready.auth.cookie)
            .headers(headers),
    )
    .await;

    assert_eq!(response.status(), StatusCode::PARTIAL_CONTENT);
    assert_eq!(response.headers()[header::CONTENT_RANGE], "bytes 0-0/15");
    assert_eq!(
        response.headers()[header::CACHE_CONTROL],
        "private, no-store"
    );
    assert_eq!(response.headers()[header::CONTENT_TYPE], "video/mp4");
    assert!(response.headers().get(header::SET_COOKIE).is_none());
    assert!(response.headers().get(header::SERVER).is_none());
    assert!(response.headers().get("x-upstream-private").is_none());
    let disposition = response.headers()[header::CONTENT_DISPOSITION]
        .to_str()
        .expect("content disposition");
    assert!(!disposition.contains('\r'));
    assert!(!disposition.contains('\n'));
    assert!(!disposition.contains("../"));
    assert_eq!(to_bytes(response.into_body(), 8).await.expect("body"), "s");

    let captures = harness.captures.lock().await;
    assert_eq!(captures.len(), 1);
    assert_eq!(captures[0].method, Method::GET);
    assert_eq!(captures[0].range.as_deref(), Some("bytes=0-0"));
    assert_eq!(
        captures[0].referer.as_deref(),
        Some("https://courses.sjtu.edu.cn")
    );
    assert_eq!(captures[0].accept_encoding.as_deref(), Some("identity"));
}

#[tokio::test]
async fn invalid_or_multiple_range_is_rejected_before_upstream() {
    let harness = harness(HarnessOptions::default()).await;
    let ready = ready_download(&harness.app).await;
    for range in ["items=0-1", "bytes=0-1,4-5"] {
        let mut headers = HeaderMap::new();
        headers.insert(header::RANGE, range.parse().expect("range"));
        let response = request(
            &harness.app,
            RequestSpec::get(&ready.download_url)
                .cookie(&ready.auth.cookie)
                .headers(headers),
        )
        .await;
        assert_eq!(response.status(), StatusCode::RANGE_NOT_SATISFIABLE);
    }
    assert!(harness.captures.lock().await.is_empty());
}

#[tokio::test]
async fn ticket_cannot_cross_sessions_and_head_has_no_body() {
    let harness = harness(HarnessOptions::default()).await;
    let ready = ready_download(&harness.app).await;
    let other = login(&harness.app).await;
    let denied = request(
        &harness.app,
        RequestSpec::get(&ready.download_url).cookie(&other.cookie),
    )
    .await;
    assert_eq!(denied.status(), StatusCode::FORBIDDEN);

    let head = request(
        &harness.app,
        RequestSpec::head(&ready.download_url).cookie(&ready.auth.cookie),
    )
    .await;
    assert_eq!(head.status(), StatusCode::OK);
    assert!(
        to_bytes(head.into_body(), 1)
            .await
            .expect("HEAD body")
            .is_empty()
    );
}

#[tokio::test]
async fn open_suffix_and_unsatisfied_ranges_are_forwarded() {
    let harness = harness(HarnessOptions::default()).await;
    let ready = ready_download(&harness.app).await;
    for range in ["bytes=1024-", "bytes=-1024"] {
        let response = range_request(&harness, &ready, range).await;
        assert_eq!(response.status(), StatusCode::PARTIAL_CONTENT);
        drop(response);
    }
    let unsatisfied = range_request(&harness, &ready, "bytes=999-").await;
    assert_eq!(unsatisfied.status(), StatusCode::RANGE_NOT_SATISFIABLE);
    assert_eq!(
        unsatisfied.headers().get(header::CONTENT_RANGE),
        Some(&"bytes */15".parse().expect("content range"))
    );
    assert_eq!(
        unsatisfied.headers().get(header::CONTENT_LENGTH),
        Some(&"0".parse().expect("content length"))
    );
    assert!(
        to_bytes(unsatisfied.into_body(), 1)
            .await
            .expect("416 body")
            .is_empty()
    );

    let captures = harness.captures.lock().await;
    let ranges = captures
        .iter()
        .filter_map(|capture| capture.range.as_deref())
        .collect::<Vec<_>>();
    assert_eq!(ranges, ["bytes=1024-", "bytes=-1024", "bytes=999-"]);
}
