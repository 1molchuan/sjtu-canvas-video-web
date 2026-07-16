use std::sync::LazyLock;

use regex::Regex;
use secrecy::{ExposeSecret, SecretString};
use serde::Deserialize;
use url::Url;

use crate::error::ProtocolError;

static UUID_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"uuid\s*[:=]\s*["']?([0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12})["']?"#,
    )
    .expect("UUID pattern must compile")
});

#[derive(Debug)]
pub enum QrEvent {
    Update(QrCodePayload),
    Login,
    Unknown { event_type: String },
}

#[derive(Debug)]
pub struct QrCodePayload {
    pub timestamp: i64,
    pub signature: SecretString,
}

#[derive(Deserialize)]
struct WireMessage {
    #[serde(rename = "type")]
    event_type: String,
    #[serde(default)]
    payload: WirePayload,
}

#[derive(Default, Deserialize)]
struct WirePayload {
    ts: Option<i64>,
    sig: Option<String>,
}

pub fn parse_uuid_from_html(html: &str) -> Result<SecretString, ProtocolError> {
    let values = UUID_PATTERN
        .captures_iter(html)
        .filter_map(|captures| captures.get(1))
        .map(|value| value.as_str().to_owned())
        .collect::<Vec<_>>();
    if values.len() != 1 {
        return Err(ProtocolError::JAccountUuidUnavailable);
    }
    Ok(SecretString::from(values[0].clone()))
}

pub fn parse_qr_message(text: &str) -> Result<QrEvent, ProtocolError> {
    let message: WireMessage =
        serde_json::from_str(text).map_err(|_| ProtocolError::JAccountMessageInvalid)?;
    match message.event_type.to_ascii_uppercase().as_str() {
        "UPDATE_QR_CODE" => parse_update(message.payload),
        "LOGIN" => Ok(QrEvent::Login),
        _ => Ok(QrEvent::Unknown {
            event_type: message.event_type,
        }),
    }
}

pub fn build_qr_url(
    base_url: &Url,
    uuid: &SecretString,
    payload: &QrCodePayload,
) -> Result<SecretString, ProtocolError> {
    let mut url = base_url.clone();
    url.query_pairs_mut()
        .append_pair("uuid", uuid.expose_secret())
        .append_pair("ts", &payload.timestamp.to_string())
        .append_pair("sig", payload.signature.expose_secret());
    Ok(SecretString::from(url.to_string()))
}

fn parse_update(payload: WirePayload) -> Result<QrEvent, ProtocolError> {
    let timestamp = payload.ts.ok_or(ProtocolError::JAccountQrPayloadMissing)?;
    let signature = payload
        .sig
        .filter(|value| !value.is_empty())
        .ok_or(ProtocolError::JAccountQrPayloadMissing)?;
    Ok(QrEvent::Update(QrCodePayload {
        timestamp,
        signature: SecretString::from(signature),
    }))
}
