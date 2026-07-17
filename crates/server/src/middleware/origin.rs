use axum::http::{HeaderMap, header};
use thiserror::Error;
use url::Url;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum OriginError {
    #[error("Origin header is missing or invalid")]
    InvalidOrigin,
    #[error("cross-site request is forbidden")]
    CrossSite,
}

pub fn verify(headers: &HeaderMap, public_origin: &Url) -> Result<(), OriginError> {
    verify_fetch_metadata(headers)?;
    let raw = headers
        .get(header::ORIGIN)
        .and_then(|value| value.to_str().ok())
        .ok_or(OriginError::InvalidOrigin)?;
    let supplied = Url::parse(raw).map_err(|_| OriginError::InvalidOrigin)?;
    if is_bare_origin(&supplied) && same_origin(&supplied, public_origin) {
        return Ok(());
    }
    Err(OriginError::InvalidOrigin)
}

fn verify_fetch_metadata(headers: &HeaderMap) -> Result<(), OriginError> {
    let Some(value) = headers
        .get("sec-fetch-site")
        .and_then(|value| value.to_str().ok())
    else {
        return Ok(());
    };
    if matches!(value, "same-origin" | "none") {
        return Ok(());
    }
    Err(OriginError::CrossSite)
}

fn is_bare_origin(url: &Url) -> bool {
    url.username().is_empty()
        && url.password().is_none()
        && matches!(url.path(), "" | "/")
        && url.query().is_none()
        && url.fragment().is_none()
}

fn same_origin(left: &Url, right: &Url) -> bool {
    left.scheme() == right.scheme()
        && left.host_str().map(str::to_ascii_lowercase)
            == right.host_str().map(str::to_ascii_lowercase)
        && left.port_or_known_default() == right.port_or_known_default()
}
