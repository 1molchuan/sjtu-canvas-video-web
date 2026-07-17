mod support;

use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use axum::{
    Router,
    extract::State,
    http::{HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
};
use canvas_core::{
    client::ProtocolContext,
    video::{
        VideoTrack, VideoTrackInput, VideoTrackKind, probe_video_track,
        probe_video_track_without_referer,
    },
};
use secrecy::SecretString;

use support::{MockServer, Shared, topology::MockTopology};

const RANGE_206: usize = 0;
const RANGE_200: usize = 1;
const RANGE_416: usize = 2;
const REDIRECT_TO_VIDEO_API: usize = 3;
const MALFORMED_206: usize = 4;
const REQUIRE_CANVAS_REFERER: usize = 5;

#[derive(Default)]
struct RangeState {
    mode: AtomicUsize,
    forbidden_target_hits: AtomicUsize,
}

#[tokio::test]
async fn range_probe_classifies_206_200_and_416_without_full_download() {
    for (mode, status, supports_range, total_size) in [
        (RANGE_206, 206, true, Some(8192)),
        (RANGE_200, 200, false, Some(8192)),
        (RANGE_416, 416, false, Some(8192)),
    ] {
        let state = Arc::new(RangeState::default());
        state.mode.store(mode, Ordering::SeqCst);
        let server = MockServer::spawn(router(state)).await;
        let topology = MockTopology::for_server(&server);
        let track = track(
            topology
                .video_content_origin
                .join("content/test.mp4")
                .unwrap(),
        );
        let context = ProtocolContext::new(topology.config).expect("context should build");

        let result = probe_video_track(&context, &track)
            .await
            .expect("Range response should be classified");

        assert_eq!(result.status, status);
        assert_eq!(result.supports_range, supports_range);
        assert_eq!(result.total_size, total_size);
    }
}

#[tokio::test]
async fn range_probe_rejects_cross_purpose_redirect_before_request() {
    let state = Arc::new(RangeState::default());
    state.mode.store(REDIRECT_TO_VIDEO_API, Ordering::SeqCst);
    let server = MockServer::spawn(router(state.clone())).await;
    let topology = MockTopology::for_server(&server);
    let track = track(
        topology
            .video_content_origin
            .join("content/test.mp4")
            .unwrap(),
    );
    let context = ProtocolContext::new(topology.config).expect("context should build");

    assert!(probe_video_track(&context, &track).await.is_err());
    assert_eq!(state.forbidden_target_hits.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn range_probe_rejects_malformed_partial_content_metadata() {
    let state = Arc::new(RangeState::default());
    state.mode.store(MALFORMED_206, Ordering::SeqCst);
    let server = MockServer::spawn(router(state)).await;
    let topology = MockTopology::for_server(&server);
    let track = track(
        topology
            .video_content_origin
            .join("content/test.mp4")
            .expect("mock content URL is valid"),
    );
    let context = ProtocolContext::new(topology.config).expect("context should build");

    assert!(probe_video_track(&context, &track).await.is_err());
}

#[tokio::test]
async fn direct_probe_omits_referer_and_reports_rejection() {
    let state = Arc::new(RangeState::default());
    state.mode.store(REQUIRE_CANVAS_REFERER, Ordering::SeqCst);
    let server = MockServer::spawn(router(state)).await;
    let topology = MockTopology::for_server(&server);
    let track = track(
        topology
            .video_content_origin
            .join("content/test.mp4")
            .expect("mock content URL is valid"),
    );
    let context = ProtocolContext::new(topology.config).expect("context should build");

    assert!(probe_video_track(&context, &track).await.is_ok());
    assert!(
        probe_video_track_without_referer(&context, &track)
            .await
            .is_err()
    );
}

#[tokio::test]
async fn direct_probe_accepts_range_when_referer_is_not_required() {
    let state = Arc::new(RangeState::default());
    let server = MockServer::spawn(router(state)).await;
    let topology = MockTopology::for_server(&server);
    let track = track(
        topology
            .video_content_origin
            .join("content/test.mp4")
            .expect("mock content URL is valid"),
    );
    let context = ProtocolContext::new(topology.config).expect("context should build");

    let result = probe_video_track_without_referer(&context, &track)
        .await
        .expect("referer-free Range response should be classified");

    assert_eq!(result.status, 206);
    assert!(result.supports_range);
}

fn router(state: Shared<RangeState>) -> Router {
    Router::new()
        .route("/content/test.mp4", get(content))
        .route("/video/forbidden", get(forbidden))
        .with_state(state)
}

async fn content(State(state): State<Shared<RangeState>>, headers: HeaderMap) -> Response {
    assert_eq!(
        headers.get("range").and_then(|value| value.to_str().ok()),
        Some("bytes=0-0")
    );
    match state.mode.load(Ordering::SeqCst) {
        RANGE_206 => response(StatusCode::PARTIAL_CONTENT, "bytes 0-0/8192", "1"),
        RANGE_200 => (StatusCode::OK, vec![0_u8; 8192]).into_response(),
        RANGE_416 => response(StatusCode::RANGE_NOT_SATISFIABLE, "bytes */8192", "0"),
        MALFORMED_206 => response(StatusCode::PARTIAL_CONTENT, "bytes 1-1/8192", "1"),
        REQUIRE_CANVAS_REFERER => require_canvas_referer(&headers),
        REDIRECT_TO_VIDEO_API => {
            let host = headers
                .get("host")
                .and_then(|value| value.to_str().ok())
                .unwrap();
            let port = host.rsplit_once(':').map_or("80", |(_, port)| port);
            let location = format!("http://video.sjtu.mock.test:{port}/video/forbidden");
            let mut response = StatusCode::FOUND.into_response();
            response.headers_mut().insert(
                "location",
                HeaderValue::from_str(&location).expect("mock redirect is valid"),
            );
            response
        }
        _ => panic!("unknown Range mode"),
    }
}

fn require_canvas_referer(headers: &HeaderMap) -> Response {
    let referer = headers.get("referer").and_then(|value| value.to_str().ok());
    if referer == Some("https://courses.sjtu.edu.cn") {
        return response(StatusCode::PARTIAL_CONTENT, "bytes 0-0/8192", "1");
    }
    StatusCode::FORBIDDEN.into_response()
}

async fn forbidden(State(state): State<Shared<RangeState>>) -> &'static str {
    state.forbidden_target_hits.fetch_add(1, Ordering::SeqCst);
    "must not be reached"
}

fn response(status: StatusCode, content_range: &str, content_length: &str) -> Response {
    let mut response = status.into_response();
    if !content_range.is_empty() {
        response.headers_mut().insert(
            "content-range",
            HeaderValue::from_str(content_range).expect("Content-Range fixture is valid"),
        );
    }
    response.headers_mut().insert(
        "content-length",
        HeaderValue::from_str(content_length).expect("Content-Length fixture is valid"),
    );
    response
}

fn track(url: url::Url) -> VideoTrack {
    VideoTrack::new(VideoTrackInput {
        id: "track-1".to_owned(),
        kind: VideoTrackKind::Unknown,
        suggested_filename: "track.mp4".to_owned(),
        upstream_url: SecretString::from(url.to_string()),
    })
}
