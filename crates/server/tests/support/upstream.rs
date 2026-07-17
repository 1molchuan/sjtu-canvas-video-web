use std::{net::SocketAddr, sync::Arc};

use axum::{
    Router,
    body::Body,
    extract::State,
    http::{HeaderMap, Method, Response, StatusCode, header},
    routing::get,
};
use futures_util::StreamExt;
use tokio::sync::Mutex;
use url::Url;

use super::CONTENT_HOST;

const BODY: &[u8] = b"synthetic-video";

pub type CaptureStore = Arc<Mutex<Vec<CapturedRequest>>>;

#[derive(Clone, Debug, Default)]
pub struct CapturedRequest {
    pub method: Method,
    pub range: Option<String>,
    pub referer: Option<String>,
    pub accept_encoding: Option<String>,
    pub origin: Option<String>,
    pub cookie: Option<String>,
    pub authorization: Option<String>,
}

pub(super) struct MockUpstream {
    pub origin: Url,
    pub address: SocketAddr,
    pub captures: CaptureStore,
}

pub(super) async fn spawn() -> MockUpstream {
    let captures = Arc::new(Mutex::new(Vec::new()));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("mock listener");
    let address = listener.local_addr().expect("mock address");
    let router = Router::new()
        .route("/video.mp4", get(video_source).head(video_source))
        .with_state(captures.clone());
    tokio::spawn(async move {
        axum::serve(listener, router).await.expect("mock upstream");
    });
    let origin =
        Url::parse(&format!("http://{CONTENT_HOST}:{}/", address.port())).expect("mock origin");
    MockUpstream {
        origin,
        address,
        captures,
    }
}

async fn video_source(
    State(captures): State<CaptureStore>,
    method: Method,
    headers: HeaderMap,
) -> Response<Body> {
    captures.lock().await.push(capture(&method, &headers));
    let range = headers
        .get(header::RANGE)
        .and_then(|value| value.to_str().ok());
    if let Some(response) = special_response(range) {
        return response;
    }
    let (status, body) = match range {
        Some("bytes=500-") => (StatusCode::INTERNAL_SERVER_ERROR, Body::empty()),
        Some(_) => (StatusCode::PARTIAL_CONTENT, Body::from("s")),
        None if method == Method::HEAD => (StatusCode::OK, Body::empty()),
        None => (StatusCode::OK, Body::from(BODY)),
    };
    upstream_response(status, range, body)
}

fn special_response(range: Option<&str>) -> Option<Response<Body>> {
    match range {
        Some("bytes=301-") => Some(redirect_response()),
        Some("bytes=700-") => Some(delayed_response(range)),
        Some("bytes=800-") => Some(interrupted_response()),
        Some("bytes=999-") => Some(unsatisfied_response()),
        _ => None,
    }
}

fn unsatisfied_response() -> Response<Body> {
    const PRIVATE_BODY: &str = "private-upstream-error";
    Response::builder()
        .status(StatusCode::RANGE_NOT_SATISFIABLE)
        .header(header::CONTENT_TYPE, "text/html")
        .header(header::CONTENT_RANGE, format!("bytes */{}", BODY.len()))
        .header(header::CONTENT_LENGTH, PRIVATE_BODY.len())
        .body(Body::from(PRIVATE_BODY))
        .expect("unsatisfied response")
}

fn redirect_response() -> Response<Body> {
    Response::builder()
        .status(StatusCode::FOUND)
        .header(header::LOCATION, "http://outside.example.test/video.mp4")
        .body(Body::empty())
        .expect("redirect response")
}

fn delayed_response(range: Option<&str>) -> Response<Body> {
    let delayed = futures_util::stream::once(async {
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        Ok::<_, std::convert::Infallible>("s")
    });
    upstream_response(
        StatusCode::PARTIAL_CONTENT,
        range,
        Body::from_stream(delayed),
    )
}

fn interrupted_response() -> Response<Body> {
    let interrupted = futures_util::stream::once(async { Ok::<_, std::io::Error>("s") }).chain(
        futures_util::stream::once(async {
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            Err(std::io::Error::new(
                std::io::ErrorKind::ConnectionReset,
                "synthetic upstream interruption",
            ))
        }),
    );
    Response::builder()
        .status(StatusCode::PARTIAL_CONTENT)
        .header(header::CONTENT_TYPE, "video/mp4")
        .header(header::CONTENT_RANGE, "bytes 0-1/15")
        .header(header::CONTENT_LENGTH, "2")
        .body(Body::from_stream(interrupted))
        .expect("interrupted response")
}

fn capture(method: &Method, headers: &HeaderMap) -> CapturedRequest {
    CapturedRequest {
        method: method.clone(),
        range: header_string(headers, header::RANGE),
        referer: header_string(headers, header::REFERER),
        accept_encoding: header_string(headers, header::ACCEPT_ENCODING),
        origin: header_string(headers, header::ORIGIN),
        cookie: header_string(headers, header::COOKIE),
        authorization: header_string(headers, header::AUTHORIZATION),
    }
}

fn header_string(headers: &HeaderMap, name: header::HeaderName) -> Option<String> {
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned)
}

fn upstream_response(status: StatusCode, range: Option<&str>, body: Body) -> Response<Body> {
    let mut response = Response::builder()
        .status(status)
        .header(header::CONTENT_TYPE, "video/mp4")
        .header(header::ACCEPT_RANGES, "bytes")
        .header(header::SET_COOKIE, "upstream=must-not-leak")
        .header(header::SERVER, "synthetic-upstream")
        .header("x-upstream-private", "must-not-leak");
    if range.is_some() && status == StatusCode::PARTIAL_CONTENT {
        response = response
            .header(header::CONTENT_RANGE, format!("bytes 0-0/{}", BODY.len()))
            .header(header::CONTENT_LENGTH, "1");
    } else {
        response = response.header(header::CONTENT_LENGTH, BODY.len().to_string());
    }
    response.body(body).expect("upstream response")
}
