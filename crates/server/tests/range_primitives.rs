use axum::http::{
    HeaderMap, HeaderValue,
    header::{
        ACCEPT_RANGES, CONNECTION, CONTENT_DISPOSITION, CONTENT_LENGTH, CONTENT_RANGE,
        CONTENT_TYPE, ETAG, LAST_MODIFIED, LOCATION, SET_COOKIE, TRANSFER_ENCODING,
    },
};
use server::stream::{
    ByteRange, MAX_FILENAME_CHARS, MAX_RANGE_HEADER_BYTES, RangeParseError,
    UPSTREAM_RESPONSE_HEADER_ALLOWLIST, attachment_content_disposition,
    filter_upstream_response_headers, is_allowed_upstream_response_header, parse_single_range,
    sanitize_filename,
};

#[test]
fn parses_closed_byte_range() {
    assert_eq!(
        parse_single_range("bytes=12-34"),
        Ok(ByteRange::Closed { start: 12, end: 34 })
    );
}

#[test]
fn parses_open_ended_byte_range() {
    assert_eq!(
        parse_single_range("bytes=12-"),
        Ok(ByteRange::OpenEnded { start: 12 })
    );
}

#[test]
fn parses_suffix_byte_range() {
    assert_eq!(
        parse_single_range("bytes=-512"),
        Ok(ByteRange::Suffix { length: 512 })
    );
}

#[test]
fn rejects_invalid_single_range_headers() {
    let cases = [
        ("", RangeParseError::Empty),
        ("bytes=", RangeParseError::Empty),
        ("bytes=-", RangeParseError::Empty),
        ("items=0-1", RangeParseError::UnsupportedUnit),
        ("bytes=0-1,2-3", RangeParseError::MultipleRanges),
        ("bytes=a-1", RangeParseError::NonDigit),
        ("bytes=0-b", RangeParseError::NonDigit),
        ("bytes=-0", RangeParseError::ZeroSuffix),
        ("bytes=18446744073709551616-", RangeParseError::Overflow),
    ];

    for (value, expected) in cases {
        assert_eq!(parse_single_range(value), Err(expected), "{value}");
    }

    let overlong = format!("bytes=0-{}", "0".repeat(MAX_RANGE_HEADER_BYTES));
    assert_eq!(parse_single_range(&overlong), Err(RangeParseError::TooLong));
}

#[test]
fn rejects_reversed_closed_range() {
    assert_eq!(
        parse_single_range("bytes=34-12"),
        Err(RangeParseError::Reversed)
    );
}

#[test]
fn sanitizes_content_disposition_filename() {
    let cleaned = sanitize_filename("..\\课程\r\n\"evil/../../录像.mp4");

    assert!(!cleaned.contains(".."));
    assert!(!cleaned.contains(['/', '\\', '\r', '\n', '"']));
    assert!(cleaned.contains("课程"));
    assert!(cleaned.contains("录像"));
    assert!(cleaned.ends_with(".mp4"));
    assert!(cleaned.chars().count() <= MAX_FILENAME_CHARS);
}

#[test]
fn dangerous_or_missing_filename_extensions_fall_back_to_mp4() {
    assert_eq!(sanitize_filename("lecture.html"), "lecture.html.mp4");
    assert_eq!(sanitize_filename(""), "video.mp4");
    assert_eq!(sanitize_filename("recording"), "recording.mp4");
    assert_eq!(sanitize_filename("recording.webm"), "recording.webm");
}

#[test]
fn builds_safe_utf8_attachment_content_disposition() {
    let value = attachment_content_disposition("课程\r\n\"../录像.mp4")
        .expect("sanitized filename should produce a valid header");
    let text = value
        .to_str()
        .expect("generated Content-Disposition should be ASCII");

    assert!(text.starts_with("attachment; filename=\""));
    assert!(text.contains("; filename*=UTF-8''"));
    assert!(text.contains("%E8%AF%BE%E7%A8%8B"));
    assert!(!text.contains(['\r', '\n', '/', '\\']));
    assert_eq!(text.matches('"').count(), 2);
}

#[test]
fn upstream_response_header_allowlist_is_narrow() {
    assert_eq!(
        UPSTREAM_RESPONSE_HEADER_ALLOWLIST,
        [
            ACCEPT_RANGES,
            CONTENT_RANGE,
            CONTENT_LENGTH,
            CONTENT_TYPE,
            LAST_MODIFIED,
            ETAG,
        ]
    );

    for name in [
        ACCEPT_RANGES,
        CONTENT_RANGE,
        CONTENT_LENGTH,
        CONTENT_TYPE,
        LAST_MODIFIED,
        ETAG,
    ] {
        assert!(is_allowed_upstream_response_header(&name), "{name}");
    }

    for name in [
        SET_COOKIE,
        LOCATION,
        CONTENT_DISPOSITION,
        CONNECTION,
        TRANSFER_ENCODING,
    ] {
        assert!(!is_allowed_upstream_response_header(&name), "{name}");
    }
}

#[test]
fn filters_upstream_response_headers_without_mutating_source() {
    let mut upstream = HeaderMap::new();
    upstream.insert(CONTENT_TYPE, HeaderValue::from_static("video/mp4"));
    upstream.insert(CONTENT_LENGTH, HeaderValue::from_static("4096"));
    upstream.insert(ETAG, HeaderValue::from_static("\"version-1\""));
    upstream.insert(SET_COOKIE, HeaderValue::from_static("secret=value"));
    upstream.insert(
        CONTENT_DISPOSITION,
        HeaderValue::from_static("attachment; filename=unsafe"),
    );

    let filtered = filter_upstream_response_headers(&upstream);

    assert_eq!(filtered.get(CONTENT_TYPE), upstream.get(CONTENT_TYPE));
    assert_eq!(filtered.get(CONTENT_LENGTH), upstream.get(CONTENT_LENGTH));
    assert_eq!(filtered.get(ETAG), upstream.get(ETAG));
    assert!(!filtered.contains_key(SET_COOKIE));
    assert!(!filtered.contains_key(CONTENT_DISPOSITION));
    assert!(upstream.contains_key(SET_COOKIE));
}
