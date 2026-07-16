use canvas_core::{
    client::{UpstreamPolicy, UpstreamPurpose, validate_upstream_url},
    redaction::Redactor,
};
use url::Url;

const TEST_REDACTION_KEY: [u8; 32] = [7; 32];

#[test]
fn production_policy_accepts_only_exact_https_host_for_purpose() {
    let policy = UpstreamPolicy::production();
    let valid =
        Url::parse("https://jaccount.sjtu.edu.cn/jaccount/sub/example").expect("valid URL fixture");
    let suffix_attack =
        Url::parse("https://jaccount.sjtu.edu.cn.evil.example/path").expect("valid URL fixture");
    let wrong_purpose =
        Url::parse("https://my.sjtu.edu.cn/api/account").expect("valid URL fixture");

    assert!(validate_upstream_url(&valid, UpstreamPurpose::JAccount, &policy).is_ok());
    assert!(validate_upstream_url(&suffix_attack, UpstreamPurpose::JAccount, &policy).is_err());
    assert!(validate_upstream_url(&wrong_purpose, UpstreamPurpose::JAccount, &policy).is_err());
}

#[test]
fn production_policy_rejects_http_credentials_ip_literals_and_custom_ports() {
    let policy = UpstreamPolicy::production();
    let fixtures = [
        "http://jaccount.sjtu.edu.cn/path",
        "https://user:password@jaccount.sjtu.edu.cn/path",
        "https://127.0.0.1/path",
        "https://jaccount.sjtu.edu.cn:8443/path",
    ];

    for fixture in fixtures {
        let url = Url::parse(fixture).expect("valid URL fixture");
        assert!(validate_upstream_url(&url, UpstreamPurpose::JAccount, &policy).is_err());
    }
}

#[test]
fn redacted_url_and_identifier_never_contain_sensitive_input() {
    let redactor = Redactor::new(TEST_REDACTION_KEY);
    let url = Url::parse(
        "https://live.sjtu.edu.cn/vod/private-course/video.mp4?token=super-secret-value",
    )
    .expect("valid URL fixture");

    let summary = redactor.sanitize_url(&url).to_string();
    let first_hash = redactor.hash_identifier("private-user-identifier");
    let second_hash = redactor.hash_identifier("private-user-identifier");

    assert!(summary.contains("host=live.sjtu.edu.cn"));
    assert!(summary.contains("query_present=true"));
    assert!(!summary.contains("private-course"));
    assert!(!summary.contains("super-secret-value"));
    assert_eq!(first_hash, second_hash);
    assert!(!first_hash.contains("private-user-identifier"));
}

#[test]
fn validation_error_does_not_echo_url_or_query() {
    let policy = UpstreamPolicy::production();
    let url = Url::parse("https://evil.example/private?token=super-secret-value")
        .expect("valid URL fixture");

    let error = validate_upstream_url(&url, UpstreamPurpose::VideoContent, &policy)
        .expect_err("unlisted host must fail");
    let rendered = error.to_string();

    assert!(!rendered.contains("evil.example/private"));
    assert!(!rendered.contains("super-secret-value"));
}
