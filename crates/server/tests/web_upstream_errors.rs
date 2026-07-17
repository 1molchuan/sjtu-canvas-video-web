mod support;

use axum::http::StatusCode;

use support::{
    GatewayMode, HarnessOptions, RequestSpec, get_json, harness, login, request, string_at,
};

#[tokio::test]
async fn course_discovery_failure_is_a_bad_gateway_not_an_empty_success() {
    let harness = harness(HarnessOptions {
        gateway_mode: GatewayMode::CoursesFail,
        ..HarnessOptions::default()
    })
    .await;
    let auth = login(&harness.app).await;
    let response = request(
        &harness.app,
        RequestSpec::get("/api/courses").cookie(&auth.cookie),
    )
    .await;
    assert_eq!(response.status(), StatusCode::BAD_GATEWAY);
}

#[tokio::test]
async fn video_catalog_failure_is_a_bad_gateway_not_an_empty_success() {
    let harness = harness(HarnessOptions {
        gateway_mode: GatewayMode::VideosFail,
        ..HarnessOptions::default()
    })
    .await;
    let auth = login(&harness.app).await;
    let courses = get_json(&harness.app, "/api/courses", &auth.cookie).await;
    let course = string_at(&courses, "/courses/0/id");
    let response = request(
        &harness.app,
        RequestSpec::get(&format!("/api/courses/{course}/videos")).cookie(&auth.cookie),
    )
    .await;
    assert_eq!(response.status(), StatusCode::BAD_GATEWAY);
}

#[tokio::test]
async fn video_detail_failure_is_a_bad_gateway_not_an_empty_success() {
    let harness = harness(HarnessOptions {
        gateway_mode: GatewayMode::DetailFail,
        ..HarnessOptions::default()
    })
    .await;
    let auth = login(&harness.app).await;
    let courses = get_json(&harness.app, "/api/courses", &auth.cookie).await;
    let course = string_at(&courses, "/courses/0/id");
    let videos_path = format!("/api/courses/{course}/videos");
    let videos = get_json(&harness.app, &videos_path, &auth.cookie).await;
    let video = string_at(&videos, "/videos/0/id");
    let response = request(
        &harness.app,
        RequestSpec::get(&format!("{videos_path}/{video}")).cookie(&auth.cookie),
    )
    .await;
    assert_eq!(response.status(), StatusCode::BAD_GATEWAY);
}
