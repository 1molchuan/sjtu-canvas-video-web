use std::collections::HashSet;

use axum::{
    body::Body,
    http::{HeaderValue, Method, Response, StatusCode, header},
};
use canvas_core::{
    client::ProtocolContext, constants::VIDEO_DOWNLOAD_REFERER, video::ValidatedUpstreamResource,
};
use tokio_util::sync::CancellationToken;
use url::Url;

use super::{
    ByteRange, DownloadPermits, StreamingBodyOptions, attachment_content_disposition,
    filter_upstream_response_headers, streaming_body,
};
use crate::error::WebError;

const MAX_DOWNLOAD_REDIRECTS: usize = 3;

pub struct ProxyOptions {
    pub method: Method,
    pub range: Option<ByteRange>,
    pub resource: ValidatedUpstreamResource,
    pub suggested_filename: String,
    pub permits: DownloadPermits,
    pub session_revoked: CancellationToken,
    pub shutting_down: CancellationToken,
}

pub async fn proxy_download(
    context: &ProtocolContext,
    options: ProxyOptions,
) -> Result<Response<Body>, WebError> {
    let response = send_upstream(context, &options).await?;
    validate_status(response.status(), &options)?;
    build_response(response, options)
}

async fn send_upstream(
    context: &ProtocolContext,
    options: &ProxyOptions,
) -> Result<reqwest::Response, WebError> {
    let mut resource = options.resource.clone();
    let mut visited = HashSet::new();
    for redirect_count in 0..=MAX_DOWNLOAD_REDIRECTS {
        let url = resource
            .validated_url(context)
            .await
            .map_err(|_| WebError::upstream_unavailable())?;
        if !visited.insert(url.to_string()) {
            return Err(WebError::upstream_unavailable());
        }
        let response = send_once(context, options, &url).await?;
        if !response.status().is_redirection() {
            return Ok(response);
        }
        if redirect_count == MAX_DOWNLOAD_REDIRECTS {
            return Err(WebError::upstream_unavailable());
        }
        resource = redirected_resource(context, &url, response.headers()).await?;
    }
    Err(WebError::upstream_unavailable())
}

async fn send_once(
    context: &ProtocolContext,
    options: &ProxyOptions,
    url: &Url,
) -> Result<reqwest::Response, WebError> {
    let mut request = context
        .streaming_client
        .request(options.method.clone(), url.clone())
        .header(header::REFERER, VIDEO_DOWNLOAD_REFERER)
        .header(header::ACCEPT_ENCODING, "identity");
    if let Some(range) = options.range {
        request = request.header(header::RANGE, range.to_string());
    }
    request
        .send()
        .await
        .map_err(|_| WebError::upstream_unavailable())
}

async fn redirected_resource(
    context: &ProtocolContext,
    current: &Url,
    headers: &reqwest::header::HeaderMap,
) -> Result<ValidatedUpstreamResource, WebError> {
    let location = headers
        .get(header::LOCATION)
        .and_then(|value| value.to_str().ok())
        .ok_or_else(WebError::upstream_unavailable)?;
    ValidatedUpstreamResource::from_redirect(context, current, location)
        .await
        .map_err(|_| WebError::upstream_unavailable())
}

fn validate_status(status: StatusCode, options: &ProxyOptions) -> Result<(), WebError> {
    if options.method == Method::GET && options.range.is_some() && status == StatusCode::OK {
        return Err(WebError::upstream_rejected_range());
    }
    match status {
        StatusCode::OK | StatusCode::PARTIAL_CONTENT | StatusCode::RANGE_NOT_SATISFIABLE => Ok(()),
        _ => Err(WebError::upstream_unavailable()),
    }
}

fn build_response(
    upstream: reqwest::Response,
    options: ProxyOptions,
) -> Result<Response<Body>, WebError> {
    let status = upstream.status();
    let mut headers = filter_upstream_response_headers(upstream.headers());
    if status == StatusCode::RANGE_NOT_SATISFIABLE {
        headers.remove(header::CONTENT_LENGTH);
    }
    headers.insert(
        header::CONTENT_DISPOSITION,
        attachment_content_disposition(&options.suggested_filename)
            .map_err(|_| WebError::internal())?,
    );
    headers.insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("private, no-store"),
    );
    headers.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    let body = response_body(upstream, options, status);
    let mut response = Response::new(body);
    *response.status_mut() = status;
    *response.headers_mut() = headers;
    Ok(response)
}

fn response_body(upstream: reqwest::Response, options: ProxyOptions, status: StatusCode) -> Body {
    if options.method == Method::HEAD || status == StatusCode::RANGE_NOT_SATISFIABLE {
        return Body::empty();
    }
    streaming_body(
        upstream,
        StreamingBodyOptions {
            permits: options.permits,
            session_revoked: options.session_revoked,
            shutting_down: options.shutting_down,
        },
    )
}
