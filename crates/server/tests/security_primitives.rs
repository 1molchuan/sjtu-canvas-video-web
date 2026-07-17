use std::time::Duration;

use axum::http::{HeaderMap, HeaderValue, header};
use canvas_core::{
    canvas::{IdentitySource, UserIdentity},
    client::{ProtocolConfig, ProtocolContext},
};
use secrecy::SecretString;
use server::{
    middleware::{cookies, csrf, origin},
    session::{UserSession, UserSessionOptions},
};
use time::OffsetDateTime;
use url::Url;

fn session(stable_id: &str) -> UserSession {
    let identity = UserIdentity {
        stable_id: SecretString::from(stable_id.to_owned()),
        account: None,
        display_name: None,
        source: IdentitySource::MySjtuAccount,
    };
    let protocol = ProtocolContext::new(ProtocolConfig::production(Duration::from_secs(1)))
        .expect("test protocol context should build");
    UserSession::new(
        identity,
        protocol,
        UserSessionOptions {
            expires_at: OffsetDateTime::now_utc() + time::Duration::hours(1),
            max_downloads: 1,
        },
    )
    .expect("session should build")
}

#[test]
fn origin_requires_exact_public_origin_and_rejects_cross_site_fetch() {
    let public_origin = Url::parse("https://canvas-video.example.test").expect("origin is valid");
    let mut headers = HeaderMap::new();
    headers.insert(
        header::ORIGIN,
        HeaderValue::from_static("https://canvas-video.example.test"),
    );
    headers.insert("sec-fetch-site", HeaderValue::from_static("same-origin"));
    assert!(origin::verify(&headers, &public_origin).is_ok());

    headers.insert("sec-fetch-site", HeaderValue::from_static("cross-site"));
    assert!(origin::verify(&headers, &public_origin).is_err());
    headers.insert("sec-fetch-site", HeaderValue::from_static("same-origin"));
    headers.insert(
        header::ORIGIN,
        HeaderValue::from_static("https://attacker.example.test"),
    );
    assert!(origin::verify(&headers, &public_origin).is_err());
}

#[test]
fn session_cookie_is_host_only_http_only_and_secure() {
    let value = "opaque-session-value";
    let options = cookies::CookieOptions {
        name: "__Host-sjtu_canvas_video_session",
        secure: true,
        same_site: cookies::SameSite::Lax,
        max_age_seconds: None,
    };
    let cookie = cookies::set_cookie(value, &options);

    assert!(cookie.contains("HttpOnly"));
    assert!(cookie.contains("Secure"));
    assert!(cookie.contains("SameSite=Lax"));
    assert!(cookie.contains("Path=/"));
    assert!(!cookie.contains("Domain="));
    assert_eq!(
        cookies::read_cookie(&format!("other=x; session={value}"), "session"),
        Some(value)
    );
}

#[test]
fn csrf_token_is_bound_to_one_session_and_compared_safely() {
    let first = session("first");
    let second = session("second");
    let first_token = csrf::token(&first);

    assert!(csrf::verify(&first, &first_token));
    assert!(!csrf::verify(&second, &first_token));
    assert!(!csrf::verify(&first, "truncated"));
}

#[tokio::test]
async fn protocol_operations_are_serialized_only_within_one_session() {
    let first = session("first");
    let second = session("second");
    let held = first.protocol_permit().await.expect("first permit");
    assert!(
        tokio::time::timeout(Duration::from_millis(10), first.protocol_permit())
            .await
            .is_err()
    );
    assert!(second.protocol_permit().await.is_some());
    drop(held);
    assert!(first.protocol_permit().await.is_some());
}
