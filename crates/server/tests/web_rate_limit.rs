mod support;

use std::net::SocketAddr;

use axum::{
    body::Body,
    extract::ConnectInfo,
    http::{Request, StatusCode, header},
};
use support::{HarnessOptions, PUBLIC_ORIGIN, harness};
use tower::ServiceExt;

const STARTS_PER_MINUTE: usize = 6;

#[tokio::test]
async fn qr_start_rate_limit_uses_direct_peer_address() {
    let harness = harness(HarnessOptions::default()).await;
    let peer: SocketAddr = "192.0.2.10:4242".parse().expect("peer address");
    for _ in 0..STARTS_PER_MINUTE {
        let response = start(&harness.app, peer).await;
        assert_eq!(response.status(), StatusCode::OK);
    }
    let limited = start(&harness.app, peer).await;
    assert_eq!(limited.status(), StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(limited.headers()[header::RETRY_AFTER], "60");

    let different_peer: SocketAddr = "192.0.2.11:4242".parse().expect("peer address");
    assert_eq!(
        start(&harness.app, different_peer).await.status(),
        StatusCode::OK
    );
}

async fn start(app: &axum::Router, peer: SocketAddr) -> axum::response::Response {
    let mut request = Request::builder()
        .method("POST")
        .uri("/api/auth/qr/start")
        .header(header::ORIGIN, PUBLIC_ORIGIN)
        .body(Body::empty())
        .expect("request");
    request.extensions_mut().insert(ConnectInfo(peer));
    app.clone().oneshot(request).await.expect("start route")
}
