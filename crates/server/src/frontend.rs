use std::{
    fs,
    path::{Path, PathBuf},
    sync::Arc,
};

use axum::{
    Extension, Router,
    body::Bytes,
    extract::Request,
    http::{HeaderMap, HeaderValue, StatusCode, header},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::get,
};
use thiserror::Error;
use tower_http::services::{ServeDir, ServeFile};

const INDEX_CACHE_CONTROL: &str = "no-cache";
const ASSET_CACHE_CONTROL: &str = "public, max-age=31536000, immutable";
const FAVICON_CACHE_CONTROL: &str = "public, max-age=86400";

#[derive(Clone)]
pub struct FrontendAssets {
    root: Arc<PathBuf>,
    index: Bytes,
}

#[derive(Debug, Error)]
pub enum FrontendError {
    #[error("failed to resolve frontend distribution directory: {0}")]
    Resolve(std::io::Error),
    #[error("frontend distribution path is not a directory")]
    NotDirectory,
    #[error("failed to read frontend index.html: {0}")]
    Index(std::io::Error),
    #[error("frontend index.html is empty")]
    EmptyIndex,
}

impl FrontendAssets {
    pub fn load(root: impl AsRef<Path>) -> Result<Self, FrontendError> {
        let root = fs::canonicalize(root).map_err(FrontendError::Resolve)?;
        if !root.is_dir() {
            return Err(FrontendError::NotDirectory);
        }
        let index = fs::read(root.join("index.html")).map_err(FrontendError::Index)?;
        if index.is_empty() {
            return Err(FrontendError::EmptyIndex);
        }
        Ok(Self {
            root: Arc::new(root),
            index: Bytes::from(index),
        })
    }
}

pub fn router<S>(assets: FrontendAssets) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    let asset_service =
        ServeDir::new(assets.root.join("assets")).append_index_html_on_directories(false);
    let asset_routes = Router::new()
        .nest_service("/assets", asset_service)
        .layer(middleware::from_fn(cache_assets));
    let favicon_routes = Router::new()
        .route_service(
            "/favicon.svg",
            ServeFile::new(assets.root.join("favicon.svg")),
        )
        .layer(middleware::from_fn(cache_favicon));

    asset_routes
        .merge(favicon_routes)
        .route("/", get(index))
        .fallback(spa_fallback)
        .layer(Extension(assets))
}

async fn index(Extension(assets): Extension<FrontendAssets>) -> Response {
    index_response(&assets)
}

async fn spa_fallback(
    Extension(assets): Extension<FrontendAssets>,
    headers: HeaderMap,
) -> Response {
    if !accepts_html(&headers) {
        return StatusCode::NOT_FOUND.into_response();
    }
    index_response(&assets)
}

fn index_response(assets: &FrontendAssets) -> Response {
    (
        [
            (
                header::CONTENT_TYPE,
                HeaderValue::from_static("text/html; charset=utf-8"),
            ),
            (
                header::CACHE_CONTROL,
                HeaderValue::from_static(INDEX_CACHE_CONTROL),
            ),
        ],
        assets.index.clone(),
    )
        .into_response()
}

fn accepts_html(headers: &HeaderMap) -> bool {
    headers
        .get(header::ACCEPT)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| {
            value.split(',').any(|item| {
                let media_type = item.split(';').next().unwrap_or_default().trim();
                matches!(media_type, "text/html" | "application/xhtml+xml")
            })
        })
}

async fn cache_assets(request: Request, next: Next) -> Response {
    cache_success(next.run(request).await, ASSET_CACHE_CONTROL)
}

async fn cache_favicon(request: Request, next: Next) -> Response {
    cache_success(next.run(request).await, FAVICON_CACHE_CONTROL)
}

fn cache_success(mut response: Response, value: &'static str) -> Response {
    if response.status().is_success() {
        response
            .headers_mut()
            .insert(header::CACHE_CONTROL, HeaderValue::from_static(value));
    }
    response
}
