use axum::http::{
    HeaderMap, HeaderName,
    header::{ACCEPT_RANGES, CONTENT_LENGTH, CONTENT_RANGE, CONTENT_TYPE, ETAG, LAST_MODIFIED},
};

pub const UPSTREAM_RESPONSE_HEADER_ALLOWLIST: [HeaderName; 6] = [
    ACCEPT_RANGES,
    CONTENT_RANGE,
    CONTENT_LENGTH,
    CONTENT_TYPE,
    LAST_MODIFIED,
    ETAG,
];

pub fn is_allowed_upstream_response_header(name: &HeaderName) -> bool {
    UPSTREAM_RESPONSE_HEADER_ALLOWLIST.contains(name)
}

pub fn filter_upstream_response_headers(upstream: &HeaderMap) -> HeaderMap {
    let mut filtered = HeaderMap::with_capacity(UPSTREAM_RESPONSE_HEADER_ALLOWLIST.len());
    for name in &UPSTREAM_RESPONSE_HEADER_ALLOWLIST {
        for value in upstream.get_all(name).iter() {
            filtered.append(name.clone(), value.clone());
        }
    }
    filtered
}
