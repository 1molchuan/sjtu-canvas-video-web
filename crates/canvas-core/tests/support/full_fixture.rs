use std::collections::HashMap;

use axum::{
    Router,
    extract::{
        Path, Query,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    http::{HeaderMap, HeaderValue, StatusCode},
    response::{Html, IntoResponse, Response},
    routing::get,
};
use futures_util::StreamExt;

use crate::{
    lti_fixture::{self, CANVAS_COURSE_ID, FlowState},
    support::{Shared, topology::redirect_url},
};

pub const UUID: &str = "123e4567-e89b-12d3-a456-426614174000";

pub fn router(flow_state: Shared<FlowState>) -> Router {
    let auth = Router::new()
        .route("/my/info", get(uuid_page))
        .route("/ja/ws/{uuid}", get(websocket))
        .route("/ja/express", get(express_login))
        .route("/ja/sso", get(jaccount_sso))
        .route("/canvas/login", get(canvas_login))
        .route("/canvas/callback", get(canvas_callback))
        .route("/canvas/protected", get(canvas_protected))
        .route("/my/account", get(my_account))
        .route("/canvas/api/self", get(canvas_self))
        .route("/canvas/api/courses", get(canvas_courses));
    auth.merge(lti_fixture::router(flow_state))
}

async fn uuid_page() -> Html<String> {
    Html(format!(r#"<script>window.app={{uuid:"{UUID}"}}</script>"#))
}

async fn websocket(Path(uuid): Path<String>, upgrade: WebSocketUpgrade) -> impl IntoResponse {
    assert_eq!(uuid, UUID);
    upgrade.on_upgrade(websocket_session)
}

async fn websocket_session(mut socket: WebSocket) {
    while let Some(Ok(message)) = socket.next().await {
        if !matches!(message, Message::Text(ref text) if text.contains("UPDATE_QR_CODE")) {
            continue;
        }
        socket
            .send(Message::Text(
                r#"{"type":"UPDATE_QR_CODE","payload":{"ts":123,"sig":"qr-canary"}}"#.into(),
            ))
            .await
            .expect("QR event should send");
        socket
            .send(Message::Text(r#"{"type":"LOGIN","payload":{}}"#.into()))
            .await
            .expect("LOGIN event should send");
        return;
    }
}

async fn express_login(Query(query): Query<HashMap<String, String>>) -> Response {
    assert_eq!(query.get("uuid").map(String::as_str), Some(UUID));
    let mut response = "ok".into_response();
    response.headers_mut().insert(
        "set-cookie",
        HeaderValue::from_static("JAAuthCookie=ja-auth-canary; Path=/; HttpOnly"),
    );
    response
}

async fn canvas_login(headers: HeaderMap) -> Response {
    assert!(!headers.contains_key("cookie"));
    redirect(redirect_url(&headers, "127.0.0.1", "/ja/sso"))
}

async fn jaccount_sso(headers: HeaderMap) -> Response {
    assert!(header_contains(&headers, "cookie", "JAAuthCookie="));
    redirect(redirect_url(
        &headers,
        "canvas.sjtu.mock.test",
        "/canvas/callback",
    ))
}

async fn canvas_callback(headers: HeaderMap) -> Response {
    let mut response = redirect(redirect_url(
        &headers,
        "canvas.sjtu.mock.test",
        "/canvas/protected",
    ));
    response.headers_mut().insert(
        "set-cookie",
        HeaderValue::from_static("CanvasSession=canvas-canary; Path=/; HttpOnly"),
    );
    response
}

async fn canvas_protected(headers: HeaderMap) -> &'static str {
    assert!(header_contains(&headers, "cookie", "CanvasSession="));
    "authenticated"
}

async fn my_account() -> &'static str {
    r#"{"data":{"uuid":"stable-user-canary","name":"Private Name"}}"#
}

async fn canvas_self() -> &'static str {
    r#"{"id":999}"#
}

async fn canvas_courses(headers: HeaderMap) -> Response {
    assert!(!headers.contains_key("authorization"));
    (
        [("content-type", "application/json")],
        format!(r#"[{{"id":{CANVAS_COURSE_ID},"name":"Mock Course","course_code":"MOCK-1"}}]"#),
    )
        .into_response()
}

fn redirect(location: String) -> Response {
    let mut response = StatusCode::FOUND.into_response();
    response.headers_mut().insert(
        "location",
        HeaderValue::from_str(&location).expect("mock redirect is valid"),
    );
    response
}

fn header_contains(headers: &HeaderMap, name: &str, expected: &str) -> bool {
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value.contains(expected))
}
