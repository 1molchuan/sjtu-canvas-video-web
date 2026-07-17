use std::collections::HashSet;

use axum::{
    body::Body,
    http::{HeaderValue, Response, StatusCode, header},
};
use canvas_core::{
    client::ProtocolContext, constants::VIDEO_DOWNLOAD_REFERER, video::ValidatedUpstreamResource,
};
use reqwest::{RequestBuilder, header::HeaderMap};
use url::Url;

use crate::error::WebError;

const MAX_PROBE_REDIRECTS: usize = 3;
const MAX_LOGGED_MIME_LENGTH: usize = 64;

pub async fn direct_download_redirect(
    context: &ProtocolContext,
    resource: &ValidatedUpstreamResource,
    public_origin: &Url,
) -> Result<Response<Body>, WebError> {
    let target = resource
        .validated_url(context)
        .await
        .map_err(|_| WebError::upstream_unavailable())?;
    run_compatibility_probes(context, resource, public_origin).await;
    build_redirect(target)
}

async fn run_compatibility_probes(
    context: &ProtocolContext,
    resource: &ValidatedUpstreamResource,
    public_origin: &Url,
) {
    for mode in [
        ProbeMode::Navigation,
        ProbeMode::Cors,
        ProbeMode::ProxyControl,
    ] {
        match probe_resource(context, resource, public_origin, mode).await {
            Ok(summary) => log_probe_summary(mode, &summary),
            Err(error_class) => tracing::warn!(
                operation = "direct_download_compatibility_probe",
                probe_mode = mode.label(),
                error_class,
                "direct download compatibility probe failed"
            ),
        }
    }
}

async fn probe_resource(
    context: &ProtocolContext,
    initial: &ValidatedUpstreamResource,
    public_origin: &Url,
    mode: ProbeMode,
) -> Result<ProbeSummary, &'static str> {
    let mut resource = initial.clone();
    let mut visited = HashSet::new();
    for redirects in 0..=MAX_PROBE_REDIRECTS {
        let target = resource
            .validated_url(context)
            .await
            .map_err(|_| "invalid_target")?;
        if !visited.insert(target.to_string()) {
            return Err("redirect_loop");
        }
        let response = send_probe(context, &target, public_origin, mode).await?;
        if !response.status().is_redirection() {
            return summarize_response(response, &target, public_origin, redirects);
        }
        if redirects == MAX_PROBE_REDIRECTS {
            return Err("too_many_redirects");
        }
        resource = redirect_resource(context, &target, response.headers()).await?;
    }
    Err("too_many_redirects")
}

async fn send_probe(
    context: &ProtocolContext,
    target: &Url,
    public_origin: &Url,
    mode: ProbeMode,
) -> Result<reqwest::Response, &'static str> {
    let request = context
        .stateless_client
        .get(target.clone())
        .header(header::RANGE, "bytes=0-0")
        .header(header::ACCEPT_ENCODING, "identity");
    apply_probe_headers(request, public_origin, mode)
        .send()
        .await
        .map_err(|_| "request_failed")
}

fn apply_probe_headers(
    request: RequestBuilder,
    public_origin: &Url,
    mode: ProbeMode,
) -> RequestBuilder {
    match mode {
        ProbeMode::Navigation => request,
        ProbeMode::Cors => {
            request.header(header::ORIGIN, public_origin.origin().ascii_serialization())
        }
        ProbeMode::ProxyControl => request.header(header::REFERER, VIDEO_DOWNLOAD_REFERER),
    }
}

async fn redirect_resource(
    context: &ProtocolContext,
    current: &Url,
    headers: &HeaderMap,
) -> Result<ValidatedUpstreamResource, &'static str> {
    let location = headers
        .get(header::LOCATION)
        .and_then(|value| value.to_str().ok())
        .ok_or("redirect_missing")?;
    ValidatedUpstreamResource::from_redirect(context, current, location)
        .await
        .map_err(|_| "invalid_redirect")
}

fn summarize_response(
    response: reqwest::Response,
    target: &Url,
    public_origin: &Url,
    redirects: usize,
) -> Result<ProbeSummary, &'static str> {
    let headers = response.headers();
    let host = target.host_str().ok_or("missing_host")?.to_owned();
    Ok(ProbeSummary {
        host,
        status: response.status().as_u16(),
        content_type: sanitized_mime(headers),
        attachment: is_attachment(headers),
        cors: classify_cors(headers, public_origin),
        supports_range: response.status() == StatusCode::PARTIAL_CONTENT,
        accept_ranges: headers.contains_key(header::ACCEPT_RANGES),
        redirects,
    })
}

fn sanitized_mime(headers: &HeaderMap) -> Option<String> {
    let raw = headers.get(header::CONTENT_TYPE)?.to_str().ok()?;
    let mime = raw.split(';').next()?.trim().to_ascii_lowercase();
    let allowed = mime
        .bytes()
        .all(|value| value.is_ascii_alphanumeric() || b"/!#$&^_.+-".contains(&value));
    (allowed && mime.len() <= MAX_LOGGED_MIME_LENGTH).then_some(mime)
}

fn is_attachment(headers: &HeaderMap) -> bool {
    headers
        .get(header::CONTENT_DISPOSITION)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| {
            value
                .trim_start()
                .to_ascii_lowercase()
                .starts_with("attachment")
        })
}

fn classify_cors(headers: &HeaderMap, public_origin: &Url) -> CorsOutcome {
    let Some(value) = headers
        .get(header::ACCESS_CONTROL_ALLOW_ORIGIN)
        .and_then(|value| value.to_str().ok())
    else {
        return CorsOutcome::Missing;
    };
    if value == "*" {
        return CorsOutcome::Wildcard;
    }
    if value == public_origin.origin().ascii_serialization() {
        return CorsOutcome::Exact;
    }
    CorsOutcome::Other
}

fn log_probe_summary(mode: ProbeMode, summary: &ProbeSummary) {
    tracing::info!(
        operation = "direct_download_compatibility_probe",
        probe_mode = mode.label(),
        host = %summary.host,
        status = summary.status,
        content_type = summary.content_type.as_deref().unwrap_or("missing"),
        content_disposition_attachment = summary.attachment,
        cors = summary.cors.label(),
        supports_range = summary.supports_range,
        accept_ranges = summary.accept_ranges,
        redirect_count = summary.redirects,
        "direct download compatibility probe completed"
    );
}

fn build_redirect(target: Url) -> Result<Response<Body>, WebError> {
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

#[derive(Clone, Copy)]
enum ProbeMode {
    Navigation,
    Cors,
    ProxyControl,
}

impl ProbeMode {
    const fn label(self) -> &'static str {
        match self {
            Self::Navigation => "navigation",
            Self::Cors => "cors",
            Self::ProxyControl => "proxy_control",
        }
    }
}

struct ProbeSummary {
    host: String,
    status: u16,
    content_type: Option<String>,
    attachment: bool,
    cors: CorsOutcome,
    supports_range: bool,
    accept_ranges: bool,
    redirects: usize,
}

enum CorsOutcome {
    Missing,
    Wildcard,
    Exact,
    Other,
}

impl CorsOutcome {
    const fn label(&self) -> &'static str {
        match self {
            Self::Missing => "missing",
            Self::Wildcard => "wildcard",
            Self::Exact => "exact",
            Self::Other => "other",
        }
    }
}
