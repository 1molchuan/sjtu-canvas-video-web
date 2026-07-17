use reqwest::{Response, header::LOCATION};
use secrecy::{ExposeSecret, SecretString};
use serde::Deserialize;
use url::Url;

use crate::{
    client::{ProtocolContext, UpstreamPurpose, read_limited_body, validate_upstream_url},
    constants::DEFAULT_EXTERNAL_TOOL_ID,
    error::ProtocolError,
};

use super::{
    FormExpectation, LtiFormKind, ParsedForm, diagnostics::LtiProbe, extract_token_id,
    parse_lti_form,
};

const MAX_LTI_HTML_BYTES: usize = 2 * 1024 * 1024;
const MAX_TOKEN_RESPONSE_BYTES: usize = 256 * 1024;
const MAX_OIDC_CANVAS_REDIRECTS: usize = 8;

#[derive(Debug)]
pub struct CourseVideoAuth {
    pub canvas_course_id: i64,
    pub video_course_id: String,
    pub token: SecretString,
}

#[derive(Deserialize)]
struct TokenResponse {
    data: Option<TokenData>,
}

#[derive(Deserialize)]
struct TokenData {
    token: Option<String>,
    params: Option<TokenParams>,
}

#[derive(Deserialize)]
struct TokenParams {
    #[serde(rename = "courId")]
    course_id: Option<String>,
}

pub async fn establish_course_video_session(
    context: &ProtocolContext,
    canvas_course_id: i64,
) -> Result<CourseVideoAuth, ProtocolError> {
    let external_url = external_tool_url(context, canvas_course_id)?;
    let external_html = get_html(context, &external_url, UpstreamPurpose::Canvas).await?;
    let oidc_form = parse_lti_form(
        &external_html,
        &external_url,
        FormExpectation {
            action: &context.endpoints.oidc_login,
            kind: LtiFormKind::OidcInitiation,
        },
    )?;
    let auth_html = submit_for_html(context, &oidc_form).await?;
    let auth_form = parse_lti_form(
        &auth_html,
        oidc_form.action(),
        FormExpectation {
            action: &context.endpoints.lti_auth,
            kind: LtiFormKind::Authorization,
        },
    )?;
    let redirect = submit_for_redirect(context, &auth_form).await?;
    let token_id = extract_token_id(&redirect)?;
    exchange_token(context, canvas_course_id, &token_id).await
}

fn external_tool_url(context: &ProtocolContext, course_id: i64) -> Result<Url, ProtocolError> {
    let path = format!("courses/{course_id}/external_tools/{DEFAULT_EXTERNAL_TOOL_ID}");
    context
        .endpoints
        .canvas_origin()
        .join(&path)
        .map_err(|_| ProtocolError::ExternalToolPageFailed)
}

async fn get_html(
    context: &ProtocolContext,
    url: &Url,
    purpose: UpstreamPurpose,
) -> Result<String, ProtocolError> {
    validate_upstream_url(url, purpose, &context.policy)?;
    let response = context
        .no_redirect_client
        .get(url.clone())
        .send()
        .await
        .map_err(|_| ProtocolError::ExternalToolPageFailed)?;
    LtiProbe::external_tool().record(&response);
    html_from_response(response, ProtocolError::ExternalToolPageFailed).await
}

async fn submit_for_html(
    context: &ProtocolContext,
    form: &ParsedForm,
) -> Result<String, ProtocolError> {
    validate_upstream_url(form.action(), UpstreamPurpose::VideoApi, &context.policy)?;
    let response = context
        .no_redirect_client
        .post(form.action().clone())
        .form(form.encoded_fields())
        .send()
        .await
        .map_err(|_| ProtocolError::LtiLaunchFailed)?;
    LtiProbe::oidc_submission().record(&response);
    if response.status().is_redirection() {
        return follow_oidc_redirect(context, response).await;
    }
    html_from_response(response, ProtocolError::LtiLaunchFailed).await
}

async fn follow_oidc_redirect(
    context: &ProtocolContext,
    initial_response: Response,
) -> Result<String, ProtocolError> {
    let mut response = initial_response;
    for _ in 0..MAX_OIDC_CANVAS_REDIRECTS {
        let target = raw_redirect_target(&response, response.url())?;
        validate_upstream_url(&target, UpstreamPurpose::Canvas, &context.policy)
            .map_err(|_| ProtocolError::LtiRedirectInvalid)?;
        response = context
            .no_redirect_client
            .get(target)
            .send()
            .await
            .map_err(|_| ProtocolError::LtiLaunchFailed)?;
        LtiProbe::oidc_redirect().record(&response);
        if response.status().is_success() {
            return html_from_response(response, ProtocolError::LtiLaunchFailed).await;
        }
        if !response.status().is_redirection() {
            return Err(ProtocolError::LtiLaunchFailed);
        }
    }
    Err(ProtocolError::LtiRedirectInvalid)
}

async fn html_from_response(
    response: Response,
    failure: ProtocolError,
) -> Result<String, ProtocolError> {
    if !response.status().is_success() {
        return Err(failure);
    }
    let body = read_limited_body(response, MAX_LTI_HTML_BYTES).await?;
    String::from_utf8(body).map_err(|_| ProtocolError::UpstreamChanged)
}

async fn submit_for_redirect(
    context: &ProtocolContext,
    form: &ParsedForm,
) -> Result<Url, ProtocolError> {
    validate_upstream_url(form.action(), UpstreamPurpose::VideoApi, &context.policy)?;
    let response = context
        .no_redirect_client
        .post(form.action().clone())
        .form(form.encoded_fields())
        .send()
        .await
        .map_err(|_| ProtocolError::LtiLaunchFailed)?;
    LtiProbe::auth_submission().record(&response);
    if !response.status().is_redirection() {
        return Err(ProtocolError::LtiRedirectMissing);
    }
    parse_redirect(&response, form.action(), context)
}

fn parse_redirect(
    response: &Response,
    base: &Url,
    context: &ProtocolContext,
) -> Result<Url, ProtocolError> {
    let target = raw_redirect_target(response, base)?;
    validate_upstream_url(&target, UpstreamPurpose::VideoApi, &context.policy)
        .map_err(|_| ProtocolError::LtiRedirectInvalid)?;
    Ok(target)
}

fn raw_redirect_target(response: &Response, base: &Url) -> Result<Url, ProtocolError> {
    let raw = response
        .headers()
        .get(LOCATION)
        .and_then(|value| value.to_str().ok())
        .ok_or(ProtocolError::LtiRedirectMissing)?;
    let target = base
        .join(raw)
        .map_err(|_| ProtocolError::LtiRedirectInvalid)?;
    Ok(target)
}

async fn exchange_token(
    context: &ProtocolContext,
    canvas_course_id: i64,
    token_id: &SecretString,
) -> Result<CourseVideoAuth, ProtocolError> {
    let target = &context.endpoints.token_exchange;
    validate_upstream_url(target, UpstreamPurpose::VideoApi, &context.policy)?;
    let response = context
        .stateless_client
        .get(target.clone())
        .query(&[("tokenId", token_id.expose_secret())])
        .send()
        .await
        .map_err(|_| ProtocolError::VideoTokenExchangeFailed)?;
    LtiProbe::token_exchange().record(&response);
    if !response.status().is_success() {
        return Err(ProtocolError::VideoTokenExchangeFailed);
    }
    let body = read_limited_body(response, MAX_TOKEN_RESPONSE_BYTES).await?;
    parse_token_response(canvas_course_id, &body)
}

fn parse_token_response(
    canvas_course_id: i64,
    body: &[u8],
) -> Result<CourseVideoAuth, ProtocolError> {
    let response: TokenResponse =
        serde_json::from_slice(body).map_err(|_| ProtocolError::VideoTokenExchangeFailed)?;
    let data = response
        .data
        .ok_or(ProtocolError::VideoTokenExchangeFailed)?;
    let token = data
        .token
        .filter(|value| !value.is_empty())
        .ok_or(ProtocolError::VideoTokenExchangeFailed)?;
    let video_course_id = data
        .params
        .and_then(|params| params.course_id)
        .filter(|value| !value.is_empty())
        .ok_or(ProtocolError::VideoCourseIdMissing)?;
    Ok(CourseVideoAuth {
        canvas_course_id,
        video_course_id,
        token: SecretString::from(token),
    })
}
