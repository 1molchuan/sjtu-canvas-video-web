mod support;

use std::{
    collections::HashMap,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    time::Duration,
};

use axum::{
    Router,
    extract::{
        Path, Query, State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    http::{HeaderMap, HeaderValue},
    response::{Html, IntoResponse, Response},
    routing::get,
};
use canvas_core::{
    ProtocolError,
    client::{ProtocolConfig, ProtocolContext},
    jaccount::{QrLoginOptions, QrLoginProgress, login_with_qr},
};
use futures_util::StreamExt;
use secrecy::ExposeSecret;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use support::{MockServer, Shared};

const UUID: &str = "123e4567-e89b-12d3-a456-426614174000";
const COOKIE_VALUE: &str = "cookie-secret-value";

#[derive(Default)]
struct MockState {
    express_calls: AtomicUsize,
    stall_websocket: AtomicBool,
    omit_cookie: AtomicBool,
}

#[tokio::test]
async fn each_protocol_context_owns_an_independent_cookie_store() {
    let state = Arc::new(MockState::default());
    let server = MockServer::spawn(mock_router(state)).await;
    let first = ProtocolContext::new(ProtocolConfig::mock(server.origin(), test_timeout()))
        .expect("first context should build");
    let second = ProtocolContext::new(ProtocolConfig::mock(server.origin(), test_timeout()))
        .expect("second context should build");

    first
        .client
        .get(first.endpoints.express_login.clone())
        .query(&[("uuid", UUID)])
        .send()
        .await
        .expect("mock express login should respond");

    assert!(
        first
            .cookie_names(&first.endpoints.jaccount_origin())
            .expect("cookie store should be readable")
            .contains(&"JAAuthCookie".to_owned())
    );
    assert!(
        second
            .cookie_names(&second.endpoints.jaccount_origin())
            .expect("cookie store should be readable")
            .is_empty()
    );
    assert!(!Arc::ptr_eq(&first.cookie_store, &second.cookie_store));
}

#[tokio::test]
async fn backend_websocket_qr_login_refreshes_once_and_expresses_once() {
    let state = Arc::new(MockState::default());
    let server = MockServer::spawn(mock_router(state.clone())).await;
    let context = ProtocolContext::new(ProtocolConfig::mock(server.origin(), test_timeout()))
        .expect("context should build");
    let (events, mut receiver) = mpsc::unbounded_channel();
    let options = QrLoginOptions {
        timeout: test_timeout(),
        refresh_interval: Duration::from_millis(50),
        cancellation: CancellationToken::new(),
    };

    let result = login_with_qr(&context, options, events).await;
    let progress = receiver.recv().await.expect("QR event should be emitted");

    let QrLoginProgress::QrReady { url } = progress else {
        panic!("expected QR-ready event");
    };
    assert!(url.expose_secret().contains("uuid=123e4567"));
    assert_eq!(state.express_calls.load(Ordering::SeqCst), 1);
    assert!(
        result
            .expect("login should succeed")
            .cookie_names
            .contains(&"JAAuthCookie".to_owned())
    );
}

#[tokio::test]
async fn websocket_timeout_does_not_call_express_login() {
    let state = Arc::new(MockState::default());
    state.stall_websocket.store(true, Ordering::SeqCst);
    let server = MockServer::spawn(mock_router(state.clone())).await;
    let context = ProtocolContext::new(ProtocolConfig::mock(server.origin(), test_timeout()))
        .expect("context should build");
    let (events, _receiver) = mpsc::unbounded_channel();

    let error = login_with_qr(
        &context,
        login_options(Duration::from_millis(20), CancellationToken::new()),
        events,
    )
    .await
    .expect_err("stalled WebSocket must time out");

    assert_eq!(error, ProtocolError::JAccountLoginTimeout);
    assert_eq!(state.express_calls.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn cancellation_closes_login_without_express_request() {
    let state = Arc::new(MockState::default());
    state.stall_websocket.store(true, Ordering::SeqCst);
    let server = MockServer::spawn(mock_router(state.clone())).await;
    let context = ProtocolContext::new(ProtocolConfig::mock(server.origin(), test_timeout()))
        .expect("context should build");
    let cancellation = CancellationToken::new();
    cancellation.cancel();
    let (events, _receiver) = mpsc::unbounded_channel();

    let error = login_with_qr(
        &context,
        login_options(test_timeout(), cancellation),
        events,
    )
    .await
    .expect_err("cancelled login must stop");

    assert_eq!(error, ProtocolError::JAccountLoginCancelled);
    assert_eq!(state.express_calls.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn missing_express_cookie_is_a_structured_failure() {
    let state = Arc::new(MockState::default());
    state.omit_cookie.store(true, Ordering::SeqCst);
    let server = MockServer::spawn(mock_router(state)).await;
    let context = ProtocolContext::new(ProtocolConfig::mock(server.origin(), test_timeout()))
        .expect("context should build");
    let (events, _receiver) = mpsc::unbounded_channel();

    let error = login_with_qr(
        &context,
        login_options(test_timeout(), CancellationToken::new()),
        events,
    )
    .await
    .expect_err("missing JAAuthCookie must fail");

    assert_eq!(error, ProtocolError::JAccountCookieMissing);
}

fn mock_router(state: Shared<MockState>) -> Router {
    Router::new()
        .route("/my/info", get(uuid_page))
        .route("/ja/express", get(express_login))
        .route("/ja/ws/{uuid}", get(websocket))
        .with_state(state)
}

async fn uuid_page() -> Html<String> {
    Html(format!(
        r#"<script>window.app = {{ uuid: "{UUID}" }};</script>"#
    ))
}

async fn express_login(
    State(state): State<Shared<MockState>>,
    Query(query): Query<HashMap<String, String>>,
) -> Response {
    assert_eq!(query.get("uuid").map(String::as_str), Some(UUID));
    state.express_calls.fetch_add(1, Ordering::SeqCst);
    if state.omit_cookie.load(Ordering::SeqCst) {
        return "ok without cookie".into_response();
    }
    let mut headers = HeaderMap::new();
    headers.insert(
        "set-cookie",
        HeaderValue::from_str(&format!("JAAuthCookie={COOKIE_VALUE}; Path=/; HttpOnly"))
            .expect("cookie header is valid"),
    );
    (headers, "ok").into_response()
}

async fn websocket(
    State(state): State<Shared<MockState>>,
    Path(uuid): Path<String>,
    upgrade: WebSocketUpgrade,
) -> impl IntoResponse {
    assert_eq!(uuid, UUID);
    upgrade.on_upgrade(move |socket| websocket_session(socket, state))
}

async fn websocket_session(mut socket: WebSocket, state: Shared<MockState>) {
    while let Some(Ok(message)) = socket.next().await {
        if matches!(message, Message::Text(ref text) if text.contains("UPDATE_QR_CODE")) {
            if state.stall_websocket.load(Ordering::SeqCst) {
                continue;
            }
            socket
                .send(Message::Text(
                    r#"{"type":"UPDATE_QR_CODE","payload":{"ts":123456,"sig":"qr-secret"}}"#.into(),
                ))
                .await
                .expect("QR update should send");
            socket
                .send(Message::Text(r#"{"type":"LOGIN","payload":{}}"#.into()))
                .await
                .expect("login event should send");
            return;
        }
    }
}

fn test_timeout() -> Duration {
    Duration::from_secs(2)
}

fn login_options(timeout: Duration, cancellation: CancellationToken) -> QrLoginOptions {
    QrLoginOptions {
        timeout,
        refresh_interval: Duration::from_secs(1),
        cancellation,
    }
}
