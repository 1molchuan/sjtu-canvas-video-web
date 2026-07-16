use secrecy::SecretString;
use serde_json::Value;
use url::Url;

use crate::{
    client::{ProtocolContext, UpstreamPurpose, read_limited_body, validate_upstream_url},
    error::ProtocolError,
};

const MAX_IDENTITY_BODY_BYTES: usize = 256 * 1024;
const STABLE_KEYS: [&str; 8] = [
    "uuid",
    "id",
    "accountId",
    "account_id",
    "loginId",
    "login_id",
    "username",
    "userId",
];
const ACCOUNT_KEYS: [&str; 4] = ["account", "loginId", "login_id", "username"];
const DISPLAY_KEYS: [&str; 3] = ["displayName", "display_name", "name"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdentitySource {
    MySjtuAccount,
    CanvasSelf,
}

#[derive(Debug)]
pub struct UserIdentity {
    pub stable_id: SecretString,
    pub account: Option<SecretString>,
    pub display_name: Option<SecretString>,
    pub source: IdentitySource,
}

struct IdentityProbe<'a> {
    url: &'a Url,
    purpose: UpstreamPurpose,
    source: IdentitySource,
}

pub async fn probe_identity(context: &ProtocolContext) -> Result<UserIdentity, ProtocolError> {
    let probes = [
        IdentityProbe {
            url: &context.endpoints.my_account,
            purpose: UpstreamPurpose::MySjtu,
            source: IdentitySource::MySjtuAccount,
        },
        IdentityProbe {
            url: &context.endpoints.canvas_self,
            purpose: UpstreamPurpose::Canvas,
            source: IdentitySource::CanvasSelf,
        },
    ];
    for probe in probes {
        if let Some(identity) = probe_one(context, probe).await? {
            return Ok(identity);
        }
    }
    Err(ProtocolError::IdentityUnavailable)
}

async fn probe_one(
    context: &ProtocolContext,
    probe: IdentityProbe<'_>,
) -> Result<Option<UserIdentity>, ProtocolError> {
    validate_upstream_url(probe.url, probe.purpose, &context.policy)?;
    let response = context
        .no_redirect_client
        .get(probe.url.clone())
        .send()
        .await
        .map_err(|_| ProtocolError::IdentityUnavailable)?;
    if !response.status().is_success() {
        return Ok(None);
    }
    let body = read_limited_body(response, MAX_IDENTITY_BODY_BYTES).await?;
    let value: Value = match serde_json::from_slice(&body) {
        Ok(value) => value,
        Err(_) => return Ok(None),
    };
    Ok(identity_from_value(&value, probe.source))
}

fn identity_from_value(value: &Value, source: IdentitySource) -> Option<UserIdentity> {
    let objects = candidate_objects(value);
    let stable_id = find_string(&objects, &STABLE_KEYS)?;
    Some(UserIdentity {
        stable_id: SecretString::from(stable_id),
        account: find_string(&objects, &ACCOUNT_KEYS).map(SecretString::from),
        display_name: find_string(&objects, &DISPLAY_KEYS).map(SecretString::from),
        source,
    })
}

fn candidate_objects(value: &Value) -> Vec<&Value> {
    [
        Some(value),
        value.get("data"),
        value.get("account"),
        value.pointer("/data/account"),
        value.get("user"),
        value.pointer("/data/user"),
    ]
    .into_iter()
    .flatten()
    .collect()
}

fn find_string(objects: &[&Value], keys: &[&str]) -> Option<String> {
    for key in keys {
        for object in objects {
            let Some(value) = object.get(*key) else {
                continue;
            };
            if let Some(candidate) = scalar_string(value) {
                return Some(candidate);
            }
        }
    }
    None
}

fn scalar_string(value: &Value) -> Option<String> {
    match value {
        Value::String(value) if !value.trim().is_empty() => Some(value.clone()),
        Value::Number(value) => Some(value.to_string()),
        _ => None,
    }
}
