mod body;
mod direct;
pub mod filename;
pub mod headers;
mod proxy;
pub mod range;

pub(crate) use body::{DownloadPermits, StreamingBodyOptions, streaming_body};
pub(crate) use direct::direct_download_redirect;
pub use filename::{
    MAX_FILENAME_CHARS, attachment_content_disposition, sanitize_filename,
    subtitle_attachment_content_disposition,
};
pub use headers::{
    UPSTREAM_RESPONSE_HEADER_ALLOWLIST, filter_upstream_response_headers,
    is_allowed_upstream_response_header,
};
pub(crate) use proxy::{ProxyOptions, proxy_download};
pub use range::{ByteRange, MAX_RANGE_HEADER_BYTES, RangeParseError, parse_single_range};
