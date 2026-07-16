use canvas_core::error::ProtocolError;
use canvas_core::lti::{FormExpectation, LtiFormKind, extract_token_id, parse_lti_form};
use secrecy::ExposeSecret;
use url::Url;

#[test]
fn form_parser_preserves_successful_controls_order_and_duplicates() {
    let base = Url::parse("https://canvas.example/courses/7/tool").expect("base URL is valid");
    let expected = Url::parse("https://video.example/oidc").expect("action URL is valid");
    let html = r#"
        <form action="https://video.example/oidc" method="post">
          <input type="hidden" name="role" value="student">
          <input type="hidden" name="role" value="observer">
          <input type="hidden" name="empty" value="">
          <input type="checkbox" name="checked" value="yes" checked>
          <input type="checkbox" name="unchecked" value="no">
          <input type="hidden" name="disabled" value="no" disabled>
          <input type="hidden" value="no-name">
        </form>
    "#;

    let form = parse_lti_form(
        html,
        &base,
        FormExpectation {
            action: &expected,
            kind: LtiFormKind::OidcInitiation,
        },
    )
    .expect("matching OIDC form should parse");

    assert_eq!(
        form.fields(),
        [
            ("role", "student"),
            ("role", "observer"),
            ("empty", ""),
            ("checked", "yes"),
        ]
    );
}

#[test]
fn form_parser_rejects_a_different_action_even_on_same_host() {
    let base = Url::parse("https://video.example/start").expect("base URL is valid");
    let expected = Url::parse("https://video.example/lti/auth").expect("action URL is valid");
    let html =
        r#"<form action="https://video.example/lti/other"><input name="x" value="y"></form>"#;

    let result = parse_lti_form(
        html,
        &base,
        FormExpectation {
            action: &expected,
            kind: LtiFormKind::Authorization,
        },
    );

    assert!(result.is_err());
}

#[test]
fn missing_forms_and_changed_actions_have_distinct_errors() {
    let base = Url::parse("https://video.example/start").expect("base URL is valid");
    let expected = Url::parse("https://video.example/expected").expect("action URL is valid");
    let expectation = FormExpectation {
        action: &expected,
        kind: LtiFormKind::Authorization,
    };

    let missing = error_from(parse_lti_form("<main>no form</main>", &base, expectation));
    let changed = error_from(parse_lti_form(
        r#"<form action="/changed"><input name="x" value="y"></form>"#,
        &base,
        expectation,
    ));

    assert_eq!(missing, ProtocolError::LtiFormMissing);
    assert_eq!(changed, ProtocolError::LtiActionInvalid);
}

fn error_from(result: Result<canvas_core::lti::ParsedForm, ProtocolError>) -> ProtocolError {
    match result {
        Ok(_) => panic!("form parsing should fail"),
        Err(error) => error,
    }
}

#[test]
fn token_id_uses_url_parsing_for_query_and_fragment_route_query() {
    let query =
        Url::parse("https://video.example/ui?tokenId=query-secret").expect("query URL is valid");
    let fragment = Url::parse("https://video.example/ui#/ivs/index?tokenId=fragment-secret")
        .expect("fragment URL is valid");

    assert_eq!(
        extract_token_id(&query)
            .expect("query token should parse")
            .expose_secret(),
        "query-secret"
    );
    assert_eq!(
        extract_token_id(&fragment)
            .expect("fragment token should parse")
            .expose_secret(),
        "fragment-secret"
    );
}

#[test]
fn token_id_rejects_missing_empty_and_duplicate_values() {
    for raw in [
        "https://video.example/ui",
        "https://video.example/ui?tokenId=",
        "https://video.example/ui?tokenId=one&tokenId=two",
    ] {
        let url = Url::parse(raw).expect("fixture URL is valid");
        assert!(extract_token_id(&url).is_err());
    }
}
