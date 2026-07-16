mod support;

use std::{sync::Arc, time::Duration};

use axum::{
    Router,
    extract::State,
    http::{HeaderMap, HeaderValue},
    response::{IntoResponse, Response},
    routing::get,
};
use canvas_core::client::{ProtocolConfig, ProtocolContext, ProtocolOrigins};
use tokio::sync::Mutex;

use support::{MockServer, Shared};

const COOKIE_VALUE: &str = "jaccount-cookie-canary";

#[derive(Default)]
struct CookieObservation {
    my_cookie_header: Mutex<Option<String>>,
}

#[tokio::test]
async fn host_only_jaccount_cookie_is_not_sent_to_my_sjtu_origin() {
    let observation = Arc::new(CookieObservation::default());
    let server = MockServer::spawn(router(observation.clone())).await;
    let jaccount = server.origin();
    let mut my_sjtu = server.origin();
    my_sjtu
        .set_host(Some("localhost"))
        .expect("localhost is a valid mock host");
    let origins = ProtocolOrigins::for_mock(jaccount.clone(), my_sjtu);
    let context = ProtocolContext::new(ProtocolConfig::mock_with_origins(
        origins,
        Duration::from_secs(2),
    ))
    .expect("context should build");

    context
        .client
        .get(context.endpoints.express_login.clone())
        .send()
        .await
        .expect("express endpoint should respond");
    context
        .client
        .get(context.endpoints.my_info.clone())
        .send()
        .await
        .expect("my.sjtu endpoint should respond");

    assert!(
        context
            .cookie_names(&context.endpoints.jaccount_origin())
            .expect("cookie store should be readable")
            .contains(&"JAAuthCookie".to_owned())
    );
    assert_eq!(*observation.my_cookie_header.lock().await, None);
}

fn router(observation: Shared<CookieObservation>) -> Router {
    Router::new()
        .route("/ja/express", get(express_login))
        .route("/my/info", get(observe_my_cookie))
        .with_state(observation)
}

async fn express_login() -> Response {
    let mut headers = HeaderMap::new();
    headers.insert(
        "set-cookie",
        HeaderValue::from_str(&format!("JAAuthCookie={COOKIE_VALUE}; Path=/; HttpOnly"))
            .expect("cookie fixture is a valid header"),
    );
    (headers, "ok").into_response()
}

async fn observe_my_cookie(
    State(observation): State<Shared<CookieObservation>>,
    headers: HeaderMap,
) -> &'static str {
    let value = headers
        .get("cookie")
        .and_then(|header| header.to_str().ok())
        .map(str::to_owned);
    *observation.my_cookie_header.lock().await = value;
    "ok"
}
