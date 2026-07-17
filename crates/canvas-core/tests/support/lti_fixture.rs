use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use axum::{
    Router,
    body::Bytes,
    extract::{Path, Query, State},
    http::{HeaderMap, HeaderValue, StatusCode},
    response::{Html, IntoResponse, Response},
    routing::{get, post},
};

use crate::support::{Shared, topology::redirect_url};

pub const CANVAS_COURSE_ID: i64 = 777;
pub const VIDEO_COURSE_ID: &str = "video-course-42";
pub const TOKEN_ID: &str = "token-id-secret-canary";
pub const VIDEO_TOKEN: &str = "video-token-secret-canary";
pub const VIDEO_TOKEN_B: &str = "video-token-b-secret-canary";

#[derive(Default)]
pub struct FlowState {
    pub stale_first_token: AtomicBool,
    pub always_stale: AtomicBool,
    pub malicious_redirect: AtomicBool,
    pub oidc_redirect_to_canvas: AtomicBool,
    pub oidc_redirect_to_content: AtomicBool,
    pub oidc_second_canvas_redirect: AtomicBool,
    pub oidc_redirect_loop: AtomicBool,
    pub missing_token_data: AtomicBool,
    pub alternate_token: AtomicBool,
    pub lti_launches: AtomicUsize,
    pub token_exchanges: AtomicUsize,
    pub list_calls: AtomicUsize,
    pub detail_calls: AtomicUsize,
    pub always_stale_detail: AtomicBool,
}

pub fn router(state: Shared<FlowState>) -> Router {
    Router::new()
        .route(
            "/courses/{course_id}/external_tools/8329",
            get(external_tool),
        )
        .route("/video/oidc/login_initiations", post(oidc))
        .route("/lti/authorize", get(oidc_authorize))
        .route("/lti/authorize/final", get(oidc_authorize_final))
        .route("/video/lti3/lti3Auth/ivs", post(lti_auth))
        .route("/video/lti3/getAccessTokenByTokenId", get(exchange))
        .route(
            "/video/directOnDemandPlay/findVodVideoList",
            post(video_list),
        )
        .route(
            "/video/directOnDemandPlay/getVodVideoInfos",
            post(video_detail),
        )
        .route("/content/recording-screen.mp4", get(video_content))
        .route("/content/recording-camera.mp4", get(video_content))
        .with_state(state)
}

async fn external_tool(
    State(state): State<Shared<FlowState>>,
    Path(course_id): Path<i64>,
    headers: HeaderMap,
) -> Html<String> {
    assert_eq!(course_id, CANVAS_COURSE_ID);
    assert!(!headers.contains_key("authorization"));
    state.lti_launches.fetch_add(1, Ordering::SeqCst);
    let action = redirect_url(
        &headers,
        "video.sjtu.mock.test",
        "/video/oidc/login_initiations",
    );
    Html(format!(
        r#"<form action="{action}" method="post"><input type="hidden" name="iss" value="canvas"><input type="hidden" name="login_hint" value="opaque-login-hint"></form>"#
    ))
}

async fn oidc(State(state): State<Shared<FlowState>>, headers: HeaderMap, body: Bytes) -> Response {
    assert_eq!(body.as_ref(), b"iss=canvas&login_hint=opaque-login-hint");
    if state.oidc_redirect_to_canvas.load(Ordering::SeqCst) {
        return oidc_redirect(&headers, "canvas.sjtu.mock.test", "/lti/authorize");
    }
    if state.oidc_redirect_to_content.load(Ordering::SeqCst) {
        return oidc_redirect(&headers, "content.sjtu.mock.test", "/lti/authorize");
    }
    oidc_form(&headers).into_response()
}

async fn oidc_authorize(State(state): State<Shared<FlowState>>, headers: HeaderMap) -> Response {
    if state.oidc_redirect_loop.load(Ordering::SeqCst) {
        return oidc_redirect(&headers, "canvas.sjtu.mock.test", "/lti/authorize");
    }
    if state.oidc_second_canvas_redirect.load(Ordering::SeqCst) {
        return oidc_redirect(&headers, "canvas.sjtu.mock.test", "/lti/authorize/final");
    }
    oidc_form(&headers).into_response()
}

async fn oidc_authorize_final(headers: HeaderMap) -> Html<String> {
    oidc_form(&headers)
}

fn oidc_form(headers: &HeaderMap) -> Html<String> {
    let action = redirect_url(headers, "video.sjtu.mock.test", "/video/lti3/lti3Auth/ivs");
    Html(format!(
        r#"<form action="{action}" method="post"><input type="hidden" name="state" value="opaque-state"><input type="hidden" name="id_token" value="opaque-id-token"></form>"#
    ))
}

fn oidc_redirect(headers: &HeaderMap, host: &str, path: &str) -> Response {
    let location = redirect_url(headers, host, path);
    let mut response = StatusCode::FOUND.into_response();
    response.headers_mut().insert(
        "location",
        HeaderValue::from_str(&location).expect("mock redirect is valid"),
    );
    response
}

async fn lti_auth(
    State(state): State<Shared<FlowState>>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    assert_eq!(
        body.as_ref(),
        b"state=opaque-state&id_token=opaque-id-token"
    );
    let host = if state.malicious_redirect.load(Ordering::SeqCst) {
        "content.sjtu.mock.test"
    } else {
        "video.sjtu.mock.test"
    };
    let location = format!(
        "{}#/ivs/index?tokenId={TOKEN_ID}",
        redirect_url(&headers, host, "/video-ui/")
    );
    let mut response = StatusCode::FOUND.into_response();
    response.headers_mut().insert(
        "location",
        HeaderValue::from_str(&location).expect("mock redirect is valid"),
    );
    response
}

async fn exchange(
    State(state): State<Shared<FlowState>>,
    Query(query): Query<std::collections::HashMap<String, String>>,
) -> String {
    assert_eq!(query.get("tokenId").map(String::as_str), Some(TOKEN_ID));
    if state.missing_token_data.load(Ordering::SeqCst) {
        return r#"{"code":"0","data":{}}"#.to_owned();
    }
    let call = state.token_exchanges.fetch_add(1, Ordering::SeqCst);
    let stale = state.always_stale.load(Ordering::SeqCst)
        || (state.stale_first_token.load(Ordering::SeqCst) && call == 0);
    let token = if stale {
        "stale-video-token"
    } else if state.alternate_token.load(Ordering::SeqCst) {
        VIDEO_TOKEN_B
    } else {
        VIDEO_TOKEN
    };
    format!(
        r#"{{"code":"0","data":{{"token":"{token}","params":{{"courId":"{VIDEO_COURSE_ID}"}}}}}}"#
    )
}

async fn video_list(
    State(state): State<Shared<FlowState>>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    state.list_calls.fetch_add(1, Ordering::SeqCst);
    assert!(String::from_utf8_lossy(&body).contains(VIDEO_COURSE_ID));
    let token = headers.get("token").and_then(|value| value.to_str().ok());
    if token == Some("stale-video-token") {
        return StatusCode::UNAUTHORIZED.into_response();
    }
    let expected = if state.alternate_token.load(Ordering::SeqCst) {
        VIDEO_TOKEN_B
    } else {
        VIDEO_TOKEN
    };
    assert_eq!(token, Some(expected));
    assert!(headers.contains_key("referer"));
    (
        [("content-type", "application/json")],
        r#"{"code":"0","data":{"records":[{"videoId":"video-abc","videoName":"Lecture 1","courseBeginTime":"2026-01-01 10:00:00","courseEndTime":"2026-01-01 11:00:00"}]}}"#,
    )
        .into_response()
}

async fn video_detail(
    State(state): State<Shared<FlowState>>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    state.detail_calls.fetch_add(1, Ordering::SeqCst);
    let token = headers.get("token").and_then(|value| value.to_str().ok());
    if token == Some("stale-video-token") || state.always_stale_detail.load(Ordering::SeqCst) {
        return StatusCode::UNAUTHORIZED.into_response();
    }
    assert_eq!(token, Some(VIDEO_TOKEN));
    assert!(String::from_utf8_lossy(&body).contains("id=video-abc"));
    let screen = redirect_url(
        &headers,
        "content.sjtu.mock.test",
        "/content/recording-screen.mp4?signature=upstream-secret",
    );
    let camera = redirect_url(
        &headers,
        "content.sjtu.mock.test",
        "/content/recording-camera.mp4?signature=upstream-secret",
    );
    format!(
        r#"{{"code":"0","data":{{"id":99,"videName":"Lecture 1","videoPlayResponseVoList":[{{"id":1,"trackType":"screen","rtmpUrlHdv":"{screen}"}},{{"id":2,"trackType":"camera","rtmpUrlHdv":"{camera}"}}]}}}}"#
    )
    .into_response()
}

async fn video_content(headers: HeaderMap) -> Response {
    assert_eq!(
        headers.get("range").and_then(|value| value.to_str().ok()),
        Some("bytes=0-0")
    );
    assert_eq!(
        headers
            .get("accept-encoding")
            .and_then(|value| value.to_str().ok()),
        Some("identity")
    );
    assert!(!headers.contains_key("cookie"));
    assert!(!headers.contains_key("authorization"));
    let mut response = (StatusCode::PARTIAL_CONTENT, [0_u8]).into_response();
    response
        .headers_mut()
        .insert("content-range", HeaderValue::from_static("bytes 0-0/4096"));
    response
}
