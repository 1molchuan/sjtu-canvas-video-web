use std::{collections::HashMap, net::IpAddr};

use reqwest::redirect::Policy;
use url::Url;

use crate::error::ProtocolError;

const HTTPS_PORT: u16 = 443;
const MAX_REDIRECTS: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UpstreamPurpose {
    JAccount,
    MySjtu,
    Canvas,
    VideoApi,
    VideoContent,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct AllowedOrigin {
    scheme: String,
    host: String,
    port: u16,
}

#[derive(Debug, Clone)]
pub struct UpstreamPolicy {
    allowed: HashMap<UpstreamPurpose, Vec<AllowedOrigin>>,
    production: bool,
}

impl UpstreamPolicy {
    pub fn production() -> Self {
        let entries = production_origins()
            .into_iter()
            .map(|(purpose, raw)| (purpose, static_url(raw)))
            .collect();
        Self::from_urls(entries, true)
    }

    pub(crate) fn from_urls(entries: Vec<(UpstreamPurpose, Url)>, production: bool) -> Self {
        let mut allowed = HashMap::new();
        for (purpose, url) in entries {
            allowed
                .entry(purpose)
                .or_insert_with(Vec::new)
                .push(AllowedOrigin::from_url(&url));
        }
        Self {
            allowed,
            production,
        }
    }

    fn accepts_origin(&self, url: &Url) -> bool {
        let candidate = AllowedOrigin::from_url(url);
        self.allowed
            .values()
            .flatten()
            .any(|origin| origin == &candidate)
    }
}

impl AllowedOrigin {
    fn from_url(url: &Url) -> Self {
        Self {
            scheme: url.scheme().to_owned(),
            host: url.host_str().unwrap_or_default().to_ascii_lowercase(),
            port: url.port_or_known_default().unwrap_or_default(),
        }
    }
}

pub fn validate_upstream_url(
    url: &Url,
    purpose: UpstreamPurpose,
    policy: &UpstreamPolicy,
) -> Result<(), ProtocolError> {
    validate_url_shape(url, purpose, policy.production)?;
    let candidate = AllowedOrigin::from_url(url);
    let accepted = policy
        .allowed
        .get(&purpose)
        .is_some_and(|origins| origins.contains(&candidate));
    if accepted {
        return Ok(());
    }
    Err(invalid_url(purpose, "origin is not allowlisted"))
}

pub(crate) fn follow_redirects(policy: &UpstreamPolicy) -> Policy {
    let policy = policy.clone();
    Policy::custom(move |attempt| {
        if attempt.previous().len() >= MAX_REDIRECTS {
            return attempt.error("redirect limit exceeded");
        }
        if policy.accepts_origin(attempt.url()) {
            return attempt.follow();
        }
        attempt.stop()
    })
}

fn validate_url_shape(
    url: &Url,
    purpose: UpstreamPurpose,
    production: bool,
) -> Result<(), ProtocolError> {
    if !url.username().is_empty() || url.password().is_some() {
        return Err(invalid_url(purpose, "embedded credentials are forbidden"));
    }
    let host = url
        .host_str()
        .ok_or_else(|| invalid_url(purpose, "host is missing"))?;
    if production && host.parse::<IpAddr>().is_ok() {
        return Err(invalid_url(purpose, "IP literals are forbidden"));
    }
    if production && !matches!(url.scheme(), "https" | "wss") {
        return Err(invalid_url(purpose, "secure scheme is required"));
    }
    if production && url.port().is_some_and(|port| port != HTTPS_PORT) {
        return Err(invalid_url(purpose, "custom port is forbidden"));
    }
    Ok(())
}

fn invalid_url(purpose: UpstreamPurpose, reason: &'static str) -> ProtocolError {
    ProtocolError::InvalidUpstreamUrl { purpose, reason }
}

fn production_origins() -> [(UpstreamPurpose, &'static str); 6] {
    [
        (UpstreamPurpose::JAccount, "https://jaccount.sjtu.edu.cn"),
        (UpstreamPurpose::JAccount, "wss://jaccount.sjtu.edu.cn"),
        (UpstreamPurpose::MySjtu, "https://my.sjtu.edu.cn"),
        (UpstreamPurpose::Canvas, "https://oc.sjtu.edu.cn"),
        (UpstreamPurpose::VideoApi, "https://v.sjtu.edu.cn"),
        (UpstreamPurpose::VideoContent, "https://live.sjtu.edu.cn"),
    ]
}

fn static_url(raw: &str) -> Url {
    Url::parse(raw).expect("static upstream origin must be a valid URL")
}
