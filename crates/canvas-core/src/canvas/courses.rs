use reqwest::{Response, header::CONTENT_TYPE};
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};

use super::course_diagnostics::{RestProbeMetadata, summarize_response_structure};

use crate::{
    client::{ProtocolContext, UpstreamPurpose, read_limited_body, validate_upstream_url},
    error::ProtocolError,
};

const MAX_COURSE_JSON_BYTES: usize = 2 * 1024 * 1024;
const MAX_DASHBOARD_BYTES: usize = 4 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CanvasTerm {
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CanvasCourse {
    pub id: i64,
    #[serde(default, alias = "shortName", alias = "originalName")]
    pub name: String,
    #[serde(default, alias = "courseCode")]
    pub course_code: String,
    #[serde(default)]
    pub term: Option<CanvasTerm>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CourseDiscoverySource {
    RestCookieSession,
    DashboardBootstrap,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CourseDiscoveryOutcome {
    Success {
        source: CourseDiscoverySource,
        courses: Vec<CanvasCourse>,
    },
    RequiresPersonalAccessToken,
    CookieSessionRejected,
    CsrfRequired,
    UnsupportedResponse,
    UpstreamChanged,
}

enum RestAttempt {
    Complete(CourseDiscoveryOutcome),
    TryDashboard,
}

pub async fn discover_courses(
    context: &ProtocolContext,
) -> Result<CourseDiscoveryOutcome, ProtocolError> {
    match discover_with_rest(context).await? {
        RestAttempt::Complete(outcome) => Ok(outcome),
        RestAttempt::TryDashboard => discover_from_dashboard(context).await,
    }
}

async fn discover_with_rest(context: &ProtocolContext) -> Result<RestAttempt, ProtocolError> {
    let target = &context.endpoints.canvas_courses;
    validate_upstream_url(target, UpstreamPurpose::Canvas, &context.policy)?;
    let response = context
        .no_redirect_client
        .get(target.clone())
        .query(&[
            ("include[]", "teachers"),
            ("include[]", "term"),
            ("per_page", "100"),
        ])
        .send()
        .await
        .map_err(|_| ProtocolError::CanvasCourseDiscoveryRejected)?;
    classify_rest_response(response).await
}

async fn classify_rest_response(response: Response) -> Result<RestAttempt, ProtocolError> {
    let metadata = RestProbeMetadata::from_response(&response);
    if response.status().is_redirection() || response.status().as_u16() == 401 {
        metadata.record(None, "authentication_redirect_or_rejection", None);
        return Ok(RestAttempt::TryDashboard);
    }
    if response.status().as_u16() == 403 {
        return classify_forbidden(response, metadata).await;
    }
    if response.status().is_server_error() {
        metadata.record(None, "server_error", None);
        return Ok(RestAttempt::Complete(
            CourseDiscoveryOutcome::UpstreamChanged,
        ));
    }
    if !response.status().is_success() || !is_json(&response) {
        let body = read_limited_body(response, MAX_COURSE_JSON_BYTES).await?;
        metadata.record(Some(body.len()), &summarize_response_structure(&body), None);
        return Ok(RestAttempt::Complete(
            CourseDiscoveryOutcome::UnsupportedResponse,
        ));
    }
    let body = read_limited_body(response, MAX_COURSE_JSON_BYTES).await?;
    let courses = match serde_json::from_slice::<Vec<CanvasCourse>>(&body) {
        Ok(courses) => courses,
        Err(_) => {
            metadata.record(Some(body.len()), &summarize_response_structure(&body), None);
            return Ok(RestAttempt::Complete(
                CourseDiscoveryOutcome::UnsupportedResponse,
            ));
        }
    };
    metadata.record(Some(body.len()), "json_course_array", Some(courses.len()));
    Ok(RestAttempt::Complete(CourseDiscoveryOutcome::Success {
        source: CourseDiscoverySource::RestCookieSession,
        courses,
    }))
}

async fn classify_forbidden(
    response: Response,
    metadata: RestProbeMetadata,
) -> Result<RestAttempt, ProtocolError> {
    let body = read_limited_body(response, MAX_COURSE_JSON_BYTES).await?;
    let summary = String::from_utf8_lossy(&body).to_ascii_lowercase();
    if summary.contains("csrf") {
        metadata.record(Some(body.len()), "csrf_required", None);
        return Ok(RestAttempt::Complete(CourseDiscoveryOutcome::CsrfRequired));
    }
    if summary.contains("personal access token") || summary.contains("oauth access token") {
        metadata.record(Some(body.len()), "access_token_required", None);
        return Ok(RestAttempt::Complete(
            CourseDiscoveryOutcome::RequiresPersonalAccessToken,
        ));
    }
    metadata.record(Some(body.len()), "forbidden", None);
    Ok(RestAttempt::TryDashboard)
}

async fn discover_from_dashboard(
    context: &ProtocolContext,
) -> Result<CourseDiscoveryOutcome, ProtocolError> {
    let target = &context.endpoints.canvas_dashboard;
    validate_upstream_url(target, UpstreamPurpose::Canvas, &context.policy)?;
    let response = context
        .no_redirect_client
        .get(target.clone())
        .send()
        .await
        .map_err(|_| ProtocolError::CanvasCourseDiscoveryRejected)?;
    if response.status().is_redirection() {
        return Ok(CourseDiscoveryOutcome::CookieSessionRejected);
    }
    if !response.status().is_success() {
        return Ok(CourseDiscoveryOutcome::UpstreamChanged);
    }
    let body = read_limited_body(response, MAX_DASHBOARD_BYTES).await?;
    parse_dashboard_courses(&body)
}

fn parse_dashboard_courses(body: &[u8]) -> Result<CourseDiscoveryOutcome, ProtocolError> {
    let html = String::from_utf8_lossy(body);
    let document = Html::parse_document(&html);
    let selector = Selector::parse("script#dashboard_cards[type='application/json']")
        .expect("static dashboard selector is valid");
    let Some(script) = document.select(&selector).next() else {
        return Ok(CourseDiscoveryOutcome::CookieSessionRejected);
    };
    let json = script.text().collect::<String>();
    let courses =
        serde_json::from_str(&json).map_err(|_| ProtocolError::CanvasCourseDiscoveryUnsupported)?;
    Ok(CourseDiscoveryOutcome::Success {
        source: CourseDiscoverySource::DashboardBootstrap,
        courses,
    })
}

fn is_json(response: &Response) -> bool {
    response
        .headers()
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value.starts_with("application/json"))
}
