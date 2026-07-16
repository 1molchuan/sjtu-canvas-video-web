use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

use reqwest::{
    Response,
    header::{
        ACCEPT_ENCODING, CONTENT_LENGTH, CONTENT_RANGE, CONTENT_TYPE, LOCATION, RANGE, REFERER,
    },
};
use secrecy::ExposeSecret;
use url::Url;

use crate::{
    client::{ProtocolContext, UpstreamPurpose, validate_upstream_url},
    constants::VIDEO_DOWNLOAD_REFERER,
    error::ProtocolError,
};

use super::VideoTrack;

const MAX_PROBE_REDIRECTS: usize = 3;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RangeProbeResult {
    pub status: u16,
    pub supports_range: bool,
    pub total_size: Option<u64>,
    pub content_type: Option<String>,
    pub host: String,
}

pub async fn probe_video_track(
    context: &ProtocolContext,
    track: &VideoTrack,
) -> Result<RangeProbeResult, ProtocolError> {
    let mut target = Url::parse(track.upstream_url.expose_secret())
        .map_err(|_| ProtocolError::RangeProbeFailed)?;
    for _ in 0..=MAX_PROBE_REDIRECTS {
        validate_upstream_url(&target, UpstreamPurpose::VideoContent, &context.policy)?;
        validate_resolved_address(context, &target).await?;
        let response = send_probe(context, &target).await?;
        if response.status().is_redirection() {
            target = redirect_target(&response, &target, context)?;
            continue;
        }
        return classify_response(response, &target);
    }
    Err(ProtocolError::RangeProbeFailed)
}

async fn send_probe(context: &ProtocolContext, target: &Url) -> Result<Response, ProtocolError> {
    context
        .stateless_client
        .get(target.clone())
        .header(RANGE, "bytes=0-0")
        .header(REFERER, VIDEO_DOWNLOAD_REFERER)
        .header(ACCEPT_ENCODING, "identity")
        .send()
        .await
        .map_err(|_| ProtocolError::RangeProbeFailed)
}

fn redirect_target(
    response: &Response,
    current: &Url,
    context: &ProtocolContext,
) -> Result<Url, ProtocolError> {
    let raw = response
        .headers()
        .get(LOCATION)
        .and_then(|value| value.to_str().ok())
        .ok_or(ProtocolError::RangeProbeFailed)?;
    let target = current
        .join(raw)
        .map_err(|_| ProtocolError::RangeProbeFailed)?;
    validate_upstream_url(&target, UpstreamPurpose::VideoContent, &context.policy)?;
    Ok(target)
}

fn classify_response(response: Response, target: &Url) -> Result<RangeProbeResult, ProtocolError> {
    let status = response.status().as_u16();
    let total_size = match status {
        206 => parse_partial_content_range(header(&response, CONTENT_RANGE))?,
        200 => parse_u64_header(&response, CONTENT_LENGTH),
        416 => parse_unsatisfied_content_range(header(&response, CONTENT_RANGE))?,
        _ => return Err(ProtocolError::RangeProbeFailed),
    };
    let host = target
        .host_str()
        .ok_or(ProtocolError::RangeProbeFailed)?
        .to_owned();
    Ok(RangeProbeResult {
        status,
        supports_range: status == 206,
        total_size,
        content_type: header(&response, CONTENT_TYPE).map(str::to_owned),
        host,
    })
}

fn parse_partial_content_range(value: Option<&str>) -> Result<Option<u64>, ProtocolError> {
    let value = value.ok_or(ProtocolError::UpstreamRangeRejected)?;
    let remainder = value
        .strip_prefix("bytes ")
        .ok_or(ProtocolError::UpstreamRangeRejected)?;
    let (range, total) = remainder
        .split_once('/')
        .ok_or(ProtocolError::UpstreamRangeRejected)?;
    if range != "0-0" {
        return Err(ProtocolError::UpstreamRangeRejected);
    }
    parse_total(total).map(Some)
}

fn parse_unsatisfied_content_range(value: Option<&str>) -> Result<Option<u64>, ProtocolError> {
    let total = value
        .and_then(|value| value.strip_prefix("bytes */"))
        .ok_or(ProtocolError::UpstreamRangeRejected)?;
    parse_total(total).map(Some)
}

fn parse_total(value: &str) -> Result<u64, ProtocolError> {
    value
        .parse()
        .map_err(|_| ProtocolError::UpstreamRangeRejected)
}

fn parse_u64_header(response: &Response, name: reqwest::header::HeaderName) -> Option<u64> {
    header(response, name).and_then(|value| value.parse().ok())
}

fn header(response: &Response, name: reqwest::header::HeaderName) -> Option<&str> {
    response
        .headers()
        .get(name)
        .and_then(|value| value.to_str().ok())
}

async fn validate_resolved_address(
    context: &ProtocolContext,
    target: &Url,
) -> Result<(), ProtocolError> {
    let host = target
        .host_str()
        .ok_or(ProtocolError::InvalidUpstreamHost)?;
    if context.dns_override(host).is_some() {
        return Ok(());
    }
    let port = target
        .port_or_known_default()
        .ok_or(ProtocolError::InvalidUpstreamHost)?;
    let addresses = tokio::net::lookup_host((host, port))
        .await
        .map_err(|_| ProtocolError::InvalidUpstreamHost)?;
    let mut found = false;
    for address in addresses {
        found = true;
        if is_forbidden_ip(address.ip()) {
            return Err(ProtocolError::InvalidUpstreamHost);
        }
    }
    if !found {
        return Err(ProtocolError::InvalidUpstreamHost);
    }
    Ok(())
}

pub fn is_forbidden_ip(address: IpAddr) -> bool {
    match address {
        IpAddr::V4(address) => forbidden_v4(address),
        IpAddr::V6(address) => forbidden_v6(address),
    }
}

fn forbidden_v4(address: Ipv4Addr) -> bool {
    let octets = address.octets();
    address.is_private()
        || address.is_loopback()
        || address.is_link_local()
        || address.is_unspecified()
        || address.is_multicast()
        || address.is_broadcast()
        || octets[0] == 0
        || (octets[0] == 100 && (64..=127).contains(&octets[1]))
        || (octets[0] == 198 && (18..=19).contains(&octets[1]))
        || octets[0] >= 240
}

fn forbidden_v6(address: Ipv6Addr) -> bool {
    address.is_loopback()
        || address.is_unspecified()
        || address.is_unique_local()
        || address.is_unicast_link_local()
        || address.is_multicast()
        || address.to_ipv4_mapped().is_some_and(forbidden_v4)
}
