use reqwest::{Response, header::LOCATION};
use secrecy::SecretString;
use url::Url;

use crate::{
    client::{HostCookie, ProtocolContext, UpstreamPurpose, validate_upstream_url},
    error::ProtocolError,
};

const JA_AUTH_COOKIE: &str = "JAAuthCookie";
const MAX_SSO_REDIRECTS: usize = 8;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanvasSessionStatus {
    pub authenticated: bool,
    pub final_host: Option<String>,
    pub cookie_names: Vec<String>,
}

pub async fn establish_canvas_session(
    context: &ProtocolContext,
    ja_auth_cookie: &SecretString,
) -> Result<CanvasSessionStatus, ProtocolError> {
    attach_ja_cookie(context, ja_auth_cookie)?;
    let final_url = follow_sso_chain(context).await?;
    probe_protected_canvas(context).await?;
    let canvas_origin = context.endpoints.canvas_origin();
    let cookie_names = context.cookie_names(&canvas_origin)?;
    if cookie_names.is_empty() {
        return Err(ProtocolError::CanvasSessionCookieMissing);
    }
    Ok(CanvasSessionStatus {
        authenticated: true,
        final_host: final_url.host_str().map(str::to_owned),
        cookie_names,
    })
}

fn attach_ja_cookie(context: &ProtocolContext, value: &SecretString) -> Result<(), ProtocolError> {
    context.remove_cookies_named(JA_AUTH_COOKIE)?;
    for target in [
        context.endpoints.jaccount_origin(),
        context.endpoints.my_sjtu_origin(),
    ] {
        context.insert_host_cookie(HostCookie {
            name: JA_AUTH_COOKIE,
            value,
            target: &target,
        })?;
    }
    Ok(())
}

async fn follow_sso_chain(context: &ProtocolContext) -> Result<Url, ProtocolError> {
    let mut current = context.endpoints.canvas_login.clone();
    for _ in 0..=MAX_SSO_REDIRECTS {
        validate_sso_url(&current, context)?;
        let response = context
            .no_redirect_client
            .get(current.clone())
            .send()
            .await
            .map_err(|_| ProtocolError::CanvasLoginFailed)?;
        if response.status().is_redirection() {
            current = redirect_target(&response, &current, context)?;
            continue;
        }
        if !response.status().is_success() {
            return Err(ProtocolError::CanvasLoginFailed);
        }
        validate_upstream_url(&current, UpstreamPurpose::Canvas, &context.policy)
            .map_err(|_| ProtocolError::CanvasRedirectedToLogin)?;
        return Ok(current);
    }
    Err(ProtocolError::CanvasRedirectLimitExceeded)
}

async fn probe_protected_canvas(context: &ProtocolContext) -> Result<(), ProtocolError> {
    let target = &context.endpoints.canvas_protected;
    validate_upstream_url(target, UpstreamPurpose::Canvas, &context.policy)?;
    let response = context
        .no_redirect_client
        .get(target.clone())
        .send()
        .await
        .map_err(|_| ProtocolError::CanvasLoginFailed)?;
    if response.status().is_redirection() {
        return Err(ProtocolError::CanvasRedirectedToLogin);
    }
    if !response.status().is_success() {
        return Err(ProtocolError::CanvasLoginFailed);
    }
    Ok(())
}

fn redirect_target(
    response: &Response,
    current: &Url,
    context: &ProtocolContext,
) -> Result<Url, ProtocolError> {
    let location = response
        .headers()
        .get(LOCATION)
        .and_then(|value| value.to_str().ok())
        .ok_or(ProtocolError::CanvasRedirectMissing)?;
    let target = current
        .join(location)
        .map_err(|_| ProtocolError::CanvasRedirectMissing)?;
    validate_sso_url(&target, context)?;
    Ok(target)
}

fn validate_sso_url(url: &Url, context: &ProtocolContext) -> Result<(), ProtocolError> {
    let accepted = [
        UpstreamPurpose::Canvas,
        UpstreamPurpose::JAccount,
        UpstreamPurpose::MySjtu,
    ]
    .into_iter()
    .any(|purpose| validate_upstream_url(url, purpose, &context.policy).is_ok());
    if accepted {
        return Ok(());
    }
    Err(ProtocolError::InvalidUpstreamUrl {
        purpose: UpstreamPurpose::Canvas,
        reason: "SSO redirect target is not allowlisted",
    })
}
