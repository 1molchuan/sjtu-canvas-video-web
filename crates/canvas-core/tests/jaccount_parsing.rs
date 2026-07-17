use canvas_core::{
    ProtocolError,
    jaccount::{QrEvent, build_qr_url, parse_qr_message, parse_uuid_from_html},
};
use secrecy::{ExposeSecret, SecretString};
use url::Url;

const UUID: &str = "123e4567-e89b-12d3-a456-426614174000";
const SECOND_UUID: &str = "223e4567-e89b-12d3-a456-426614174001";

#[test]
fn extracts_exactly_one_uuid_from_reference_style_html() {
    let html = format!(r#"<script>window.app = {{ uuid: "{UUID}" }};</script>"#);

    let parsed = parse_uuid_from_html(&html).expect("single UUID should parse");

    assert_eq!(parsed.expose_secret(), UUID);
}

#[test]
fn missing_or_ambiguous_uuid_is_not_accepted() {
    let ambiguous = format!(r#"uuid="{UUID}"; other_uuid="{SECOND_UUID}";"#);

    assert!(matches!(
        parse_uuid_from_html("<html>no identifier</html>"),
        Err(ProtocolError::JAccountUuidUnavailable)
    ));
    assert!(matches!(
        parse_uuid_from_html(&ambiguous),
        Err(ProtocolError::JAccountUuidUnavailable)
    ));
}

#[test]
fn repeated_identical_uuid_is_accepted_once() {
    let html = format!(r#"uuid="{UUID}"; other_uuid="{UUID}"; uuid: '{UUID}';"#);

    let parsed = parse_uuid_from_html(&html).expect("identical UUID copies should be deduplicated");

    assert_eq!(parsed.expose_secret(), UUID);
}

#[test]
fn parses_update_login_and_unknown_websocket_events() {
    let update = parse_qr_message(
        r#"{"type":"UPDATE_QR_CODE","error":0,"payload":{"ts":123456,"sig":"qr-secret"}}"#,
    )
    .expect("update event should parse");
    let login = parse_qr_message(r#"{"type":"LOGIN","error":0,"payload":{}}"#)
        .expect("login event should parse");
    let unknown = parse_qr_message(r#"{"type":"SOMETHING_NEW","error":0,"payload":{}}"#)
        .expect("unknown event should remain compatible");

    match update {
        QrEvent::Update(payload) => {
            assert_eq!(payload.timestamp, 123456);
            assert_eq!(payload.signature.expose_secret(), "qr-secret");
        }
        other => panic!("unexpected event: {other:?}"),
    }
    assert!(matches!(login, QrEvent::Login));
    assert!(matches!(
        unknown,
        QrEvent::Unknown { event_type } if event_type == "SOMETHING_NEW"
    ));
}

#[test]
fn parses_expiry_and_rejects_nonzero_upstream_error() {
    let expired = parse_qr_message(r#"{"type":"QR_CODE_EXPIRED","error":0,"payload":{}}"#)
        .expect("expiry event should parse");
    let upstream_error = parse_qr_message(
        r#"{"type":"UPDATE_QR_CODE","error":17,"payload":{"ts":1,"sig":"secret"}}"#,
    );

    assert!(matches!(expired, QrEvent::Expired));
    assert_eq!(
        upstream_error.expect_err("nonzero error must fail"),
        ProtocolError::JAccountMessageInvalid
    );
}

#[test]
fn builds_qr_url_without_exposing_secret_through_debug() {
    let base = Url::parse("https://jaccount.sjtu.edu.cn/jaccount/confirmscancode")
        .expect("valid QR base URL");
    let uuid = SecretString::from(UUID.to_owned());
    let event =
        parse_qr_message(r#"{"type":"UPDATE_QR_CODE","payload":{"ts":123456,"sig":"qr-secret"}}"#)
            .expect("update event should parse");
    let QrEvent::Update(payload) = event else {
        panic!("expected update event");
    };

    let qr_url = build_qr_url(&base, &uuid, &payload).expect("QR URL should build");

    assert!(qr_url.expose_secret().contains("uuid=123e4567"));
    assert!(qr_url.expose_secret().contains("ts=123456"));
    assert!(qr_url.expose_secret().contains("sig=qr-secret"));
    assert!(!format!("{qr_url:?}").contains("qr-secret"));
}
