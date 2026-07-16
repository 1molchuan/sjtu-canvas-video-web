use scraper::{ElementRef, Html, Selector};
use url::Url;

use crate::error::ProtocolError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LtiFormKind {
    OidcInitiation,
    Authorization,
}

#[derive(Clone, Copy)]
pub struct FormExpectation<'a> {
    pub action: &'a Url,
    pub kind: LtiFormKind,
}

pub struct ParsedForm {
    action: Url,
    fields: Vec<(String, String)>,
}

impl ParsedForm {
    pub fn fields(&self) -> Vec<(&str, &str)> {
        self.fields
            .iter()
            .map(|(name, value)| (name.as_str(), value.as_str()))
            .collect()
    }

    pub(crate) fn action(&self) -> &Url {
        &self.action
    }

    pub(crate) fn encoded_fields(&self) -> &[(String, String)] {
        &self.fields
    }
}

pub fn parse_lti_form(
    html: &str,
    base_url: &Url,
    expectation: FormExpectation<'_>,
) -> Result<ParsedForm, ProtocolError> {
    let document = Html::parse_document(html);
    let selector = Selector::parse("form").expect("static form selector is valid");
    let mut found_form = false;
    for form in document.select(&selector) {
        found_form = true;
        let Some(action) = resolve_action(form, base_url) else {
            continue;
        };
        if action != *expectation.action || !uses_post(form) {
            continue;
        }
        return Ok(ParsedForm {
            action,
            fields: successful_inputs(form),
        });
    }
    Err(form_error(expectation.kind, found_form))
}

fn resolve_action(form: ElementRef<'_>, base_url: &Url) -> Option<Url> {
    let raw = form.value().attr("action")?;
    base_url.join(raw).ok()
}

fn uses_post(form: ElementRef<'_>) -> bool {
    form.value()
        .attr("method")
        .is_none_or(|method| method.eq_ignore_ascii_case("post"))
}

fn successful_inputs(form: ElementRef<'_>) -> Vec<(String, String)> {
    let selector = Selector::parse("input").expect("static input selector is valid");
    form.select(&selector)
        .filter(|input| is_successful(*input))
        .filter_map(|input| {
            let name = input.value().attr("name")?.to_owned();
            let value = input.value().attr("value").unwrap_or_default().to_owned();
            Some((name, value))
        })
        .collect()
}

fn is_successful(input: ElementRef<'_>) -> bool {
    if input.value().attr("disabled").is_some() || input.value().attr("name").is_none() {
        return false;
    }
    let input_type = input.value().attr("type").unwrap_or("text");
    if matches!(input_type, "submit" | "button" | "reset" | "file" | "image") {
        return false;
    }
    !matches!(input_type, "checkbox" | "radio") || input.value().attr("checked").is_some()
}

fn form_error(kind: LtiFormKind, found_form: bool) -> ProtocolError {
    match (kind, found_form) {
        (LtiFormKind::OidcInitiation, true) => ProtocolError::OidcActionInvalid,
        (LtiFormKind::OidcInitiation, false) => ProtocolError::OidcFormMissing,
        (LtiFormKind::Authorization, true) => ProtocolError::LtiActionInvalid,
        (LtiFormKind::Authorization, false) => ProtocolError::LtiFormMissing,
    }
}
