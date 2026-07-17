use axum::{
    body::Body,
    http::{HeaderValue, Response, StatusCode, header},
};
use canvas_core::{client::ProtocolContext, video::ValidatedUpstreamResource};

use crate::error::WebError;

pub async fn direct_download_redirect(
    context: &ProtocolContext,
    resource: &ValidatedUpstreamResource,
) -> Result<Response<Body>, WebError> {
    let target = resource
        .validated_url(context)
        .await
        .map_err(|_| WebError::upstream_unavailable())?;
    let location = HeaderValue::from_str(target.as_str()).map_err(|_| WebError::internal())?;
    let mut response = Response::new(Body::empty());
    *response.status_mut() = StatusCode::TEMPORARY_REDIRECT;
    response.headers_mut().insert(header::LOCATION, location);
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("private, no-store"),
    );
    response.headers_mut().insert(
        header::REFERRER_POLICY,
        HeaderValue::from_static("no-referrer"),
    );
    response.headers_mut().insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    Ok(response)
}
