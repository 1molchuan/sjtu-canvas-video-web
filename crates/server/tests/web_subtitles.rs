mod support;

use axum::{
    body::to_bytes,
    http::{StatusCode, header},
};

use support::{
    GatewayMode, HarnessOptions, RequestSpec, get_json, harness, login, request, string_at,
};

#[tokio::test]
async fn downloads_session_bound_srt_without_exposing_upstream_auth() {
    let harness = harness(HarnessOptions::default()).await;
    let auth = login(&harness.app).await;
    let path = subtitle_path(&harness.app, &auth.cookie).await;
    let response = request(&harness.app, RequestSpec::get(&path).cookie(&auth.cookie)).await;

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers().get(header::CONTENT_TYPE),
        Some(
            &"application/x-subrip; charset=utf-8"
                .parse()
                .expect("header")
        )
    );
    let disposition = response
        .headers()
        .get(header::CONTENT_DISPOSITION)
        .and_then(|value| value.to_str().ok())
        .expect("content disposition");
    assert!(disposition.starts_with("attachment;"));
    assert!(disposition.contains(".srt"));
    let body = to_bytes(response.into_body(), 64 * 1024)
        .await
        .expect("subtitle body");
    let text = String::from_utf8(body.to_vec()).expect("UTF-8 subtitle");
    assert!(text.contains("Synthetic subtitle"));
    assert!(!text.contains("synthetic-token"));
}

#[tokio::test]
async fn subtitle_handle_is_rejected_for_another_session() {
    let harness = harness(HarnessOptions::default()).await;
    let first = login(&harness.app).await;
    let path = subtitle_path(&harness.app, &first.cookie).await;
    let second = login(&harness.app).await;

    let response = request(&harness.app, RequestSpec::get(&path).cookie(&second.cookie)).await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn missing_subtitle_is_an_explicit_not_found_error() {
    let harness = harness(HarnessOptions {
        gateway_mode: GatewayMode::SubtitleMissing,
        ..HarnessOptions::default()
    })
    .await;
    let auth = login(&harness.app).await;
    let path = subtitle_path(&harness.app, &auth.cookie).await;

    let response = request(&harness.app, RequestSpec::get(&path).cookie(&auth.cookie)).await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

async fn subtitle_path(app: &axum::Router, cookie: &str) -> String {
    let courses = get_json(app, "/api/courses", cookie).await;
    let course = string_at(&courses, "/courses/0/id");
    let videos_path = format!("/api/courses/{course}/videos");
    let videos = get_json(app, &videos_path, cookie).await;
    let video = string_at(&videos, "/videos/0/id");
    format!("{videos_path}/{video}/subtitle")
}
