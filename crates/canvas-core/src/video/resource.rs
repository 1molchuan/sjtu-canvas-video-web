use secrecy::{ExposeSecret, SecretString};
use url::Url;

use crate::{
    client::{ProtocolContext, UpstreamPurpose, validate_upstream_url},
    error::ProtocolError,
};

use super::{VideoTrack, probe::validate_resolved_address};

#[derive(Clone)]
pub struct ValidatedUpstreamResource {
    url: SecretString,
}

impl ValidatedUpstreamResource {
    pub async fn from_track(
        context: &ProtocolContext,
        track: &VideoTrack,
    ) -> Result<Self, ProtocolError> {
        let url = parse_url(&track.upstream_url)?;
        Self::from_url(context, url).await
    }

    pub async fn from_redirect(
        context: &ProtocolContext,
        current: &Url,
        location: &str,
    ) -> Result<Self, ProtocolError> {
        let target = current
            .join(location)
            .map_err(|_| ProtocolError::InvalidUpstreamHost)?;
        Self::from_url(context, target).await
    }

    pub async fn validated_url(&self, context: &ProtocolContext) -> Result<Url, ProtocolError> {
        let url = parse_url(&self.url)?;
        validate_target(context, &url).await?;
        Ok(url)
    }

    async fn from_url(context: &ProtocolContext, url: Url) -> Result<Self, ProtocolError> {
        validate_target(context, &url).await?;
        Ok(Self {
            url: SecretString::from(url.to_string()),
        })
    }
}

impl std::fmt::Debug for ValidatedUpstreamResource {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("ValidatedUpstreamResource(<redacted>)")
    }
}

fn parse_url(value: &SecretString) -> Result<Url, ProtocolError> {
    Url::parse(value.expose_secret()).map_err(|_| ProtocolError::InvalidUpstreamHost)
}

async fn validate_target(context: &ProtocolContext, url: &Url) -> Result<(), ProtocolError> {
    validate_upstream_url(url, UpstreamPurpose::VideoContent, &context.policy)?;
    validate_resolved_address(context, url).await
}
