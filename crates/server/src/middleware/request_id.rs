use axum::{
    extract::Request,
    http::{HeaderValue, header::HeaderName},
    middleware::Next,
    response::Response,
};
use uuid::Uuid;

static REQUEST_ID_HEADER: HeaderName = HeaderName::from_static("x-request-id");

tokio::task_local! {
    static REQUEST_ID: String;
}

pub async fn apply(request: Request, next: Next) -> Response {
    let request_id = Uuid::new_v4().to_string();
    REQUEST_ID
        .scope(request_id.clone(), async move {
            let mut response = next.run(request).await;
            if let Ok(value) = HeaderValue::from_str(&request_id) {
                response
                    .headers_mut()
                    .insert(REQUEST_ID_HEADER.clone(), value);
            }
            response
        })
        .await
}

pub fn current() -> String {
    REQUEST_ID
        .try_with(Clone::clone)
        .unwrap_or_else(|_| Uuid::new_v4().to_string())
}
