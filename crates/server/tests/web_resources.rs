mod support;

use axum::http::StatusCode;

use support::{HarnessOptions, RequestSpec, get_json, harness, login, post, request, string_at};

const REAL_COURSE_ID_CANARY: &str = "4242";
const REAL_VIDEO_ID_CANARY: &str = "synthetic-video-id";
const UPSTREAM_CANARY: &str = "credential=synthetic";

#[tokio::test]
async fn resource_handles_hide_real_ids_and_are_session_bound() {
    let harness = harness(HarnessOptions::default()).await;
    let auth_a = login(&harness.app).await;
    let courses = get_json(&harness.app, "/api/courses", &auth_a.cookie).await;
    let course = string_at(&courses, "/courses/0/id");
    assert_hidden(&courses);

    let videos_path = format!("/api/courses/{course}/videos");
    let videos = get_json(&harness.app, &videos_path, &auth_a.cookie).await;
    let video = string_at(&videos, "/videos/0/id");
    assert_hidden(&videos);

    let detail_path = format!("{videos_path}/{video}");
    let detail = get_json(&harness.app, &detail_path, &auth_a.cookie).await;
    let track = string_at(&detail, "/video/tracks/0/id");
    assert_hidden(&detail);

    let ticket_path = format!("{detail_path}/tracks/{track}/ticket");
    let ticket = post(&harness.app, &ticket_path, &auth_a).await;
    assert!(ticket["download_url"].as_str().is_some());
    assert_hidden(&ticket);

    let auth_b = login(&harness.app).await;
    let denied = request(
        &harness.app,
        RequestSpec::get(&videos_path).cookie(&auth_b.cookie),
    )
    .await;
    assert_eq!(denied.status(), StatusCode::NOT_FOUND);
}

fn assert_hidden(value: &serde_json::Value) {
    let serialized = value.to_string();
    for canary in [REAL_COURSE_ID_CANARY, REAL_VIDEO_ID_CANARY, UPSTREAM_CANARY] {
        assert!(!serialized.contains(canary));
    }
}
