mod support;

use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicUsize, Ordering},
};

use axum::{
    Router,
    extract::{RawQuery, State},
    http::{HeaderMap, StatusCode},
    response::{Html, IntoResponse, Response},
    routing::get,
};
use canvas_core::{
    canvas::{
        CourseDiscoveryOutcome, CourseDiscoverySource, IdentitySource, discover_courses,
        probe_identity,
    },
    client::ProtocolContext,
    error::ProtocolError,
};
use secrecy::ExposeSecret;

use support::{MockServer, Shared, topology::MockTopology};

#[derive(Default)]
struct DiscoveryState {
    course_mode: AtomicUsize,
    identity_unavailable: AtomicBool,
    saw_authorization: AtomicBool,
    saw_course_query: AtomicBool,
}

const REST_SUCCESS: usize = 0;
const REST_REJECT_DASHBOARD_SUCCESS: usize = 1;
const REST_CSRF_REQUIRED: usize = 2;
const REST_PAT_REQUIRED: usize = 3;
const REST_MALFORMED: usize = 4;
const REST_REJECT_DASHBOARD_EMPTY: usize = 5;
const REST_OPTIONAL_FIELDS_MISSING: usize = 6;

#[tokio::test]
async fn identity_uses_canvas_numeric_id_not_display_name() {
    let state = Arc::new(DiscoveryState::default());
    let server = MockServer::spawn(router(state)).await;
    let context = context(&server);

    let identity = probe_identity(&context)
        .await
        .expect("Canvas self endpoint supplies a stable ID");

    assert_eq!(identity.stable_id.expose_secret(), "4242");
    assert_eq!(identity.source, IdentitySource::CanvasSelf);
    assert_ne!(identity.stable_id.expose_secret(), "Display Name Only");
}

#[tokio::test]
async fn identity_does_not_promote_a_display_name_to_stable_id() {
    let state = Arc::new(DiscoveryState::default());
    state.identity_unavailable.store(true, Ordering::SeqCst);
    let server = MockServer::spawn(router(state)).await;
    let context = context(&server);

    let error = probe_identity(&context)
        .await
        .expect_err("display names are not stable identity fields");

    assert_eq!(error, ProtocolError::IdentityUnavailable);
}

#[tokio::test]
async fn cookie_rest_course_discovery_sends_no_authorization() {
    let state = Arc::new(DiscoveryState::default());
    let server = MockServer::spawn(router(state.clone())).await;
    let context = context(&server);

    let outcome = discover_courses(&context)
        .await
        .expect("mock REST course discovery should complete");
    let CourseDiscoveryOutcome::Success { source, courses } = outcome else {
        panic!("expected successful REST discovery");
    };

    assert_eq!(source, CourseDiscoverySource::RestCookieSession);
    assert_eq!(courses.len(), 1);
    assert_eq!(courses[0].id, 101);
    assert!(state.saw_course_query.load(Ordering::SeqCst));
    assert!(!state.saw_authorization.load(Ordering::SeqCst));
}

#[tokio::test]
async fn cookie_rest_course_discovery_accepts_missing_display_fields() {
    let state = Arc::new(DiscoveryState::default());
    state
        .course_mode
        .store(REST_OPTIONAL_FIELDS_MISSING, Ordering::SeqCst);
    let server = MockServer::spawn(router(state)).await;

    let outcome = discover_courses(&context(&server))
        .await
        .expect("missing display fields should not invalidate the course array");
    let CourseDiscoveryOutcome::Success { courses, .. } = outcome else {
        panic!("expected successful REST discovery");
    };

    assert_eq!(courses.len(), 2);
    assert_eq!(courses[0].name, "Named course");
    assert!(courses[1].name.is_empty());
}

#[tokio::test]
async fn rejected_rest_can_use_explicit_dashboard_bootstrap() {
    let state = Arc::new(DiscoveryState::default());
    state
        .course_mode
        .store(REST_REJECT_DASHBOARD_SUCCESS, Ordering::SeqCst);
    let server = MockServer::spawn(router(state.clone())).await;
    let context = context(&server);

    let outcome = discover_courses(&context)
        .await
        .expect("dashboard experiment should complete");
    let CourseDiscoveryOutcome::Success { source, courses } = outcome else {
        panic!("expected dashboard bootstrap discovery");
    };

    assert_eq!(source, CourseDiscoverySource::DashboardBootstrap);
    assert_eq!(courses[0].id, 202);
    assert!(!state.saw_authorization.load(Ordering::SeqCst));
}

#[tokio::test]
async fn course_discovery_keeps_csrf_pat_and_unsupported_results_distinct() {
    let cases = [
        (REST_CSRF_REQUIRED, CourseDiscoveryOutcome::CsrfRequired),
        (
            REST_PAT_REQUIRED,
            CourseDiscoveryOutcome::RequiresPersonalAccessToken,
        ),
        (REST_MALFORMED, CourseDiscoveryOutcome::UnsupportedResponse),
        (
            REST_REJECT_DASHBOARD_EMPTY,
            CourseDiscoveryOutcome::CookieSessionRejected,
        ),
    ];
    for (mode, expected) in cases {
        let state = Arc::new(DiscoveryState::default());
        state.course_mode.store(mode, Ordering::SeqCst);
        let server = MockServer::spawn(router(state)).await;
        let outcome = discover_courses(&context(&server))
            .await
            .expect("course discovery experiment should classify its response");
        assert_eq!(outcome, expected);
    }
}

fn context(server: &MockServer) -> ProtocolContext {
    ProtocolContext::new(MockTopology::for_server(server).config)
        .expect("mock protocol context should build")
}

fn router(state: Shared<DiscoveryState>) -> Router {
    Router::new()
        .route("/my/account", get(my_account))
        .route("/canvas/api/self", get(canvas_self))
        .route("/canvas/api/courses", get(canvas_courses))
        .route("/canvas/dashboard", get(canvas_dashboard))
        .with_state(state)
}

async fn my_account() -> &'static str {
    r#"{"name":"Display Name Only"}"#
}

async fn canvas_self(State(state): State<Shared<DiscoveryState>>) -> &'static str {
    if state.identity_unavailable.load(Ordering::SeqCst) {
        return r#"{"name":"Another Display Name"}"#;
    }
    r#"{"id":4242,"name":"Private Name","login_id":"private-account"}"#
}

async fn canvas_courses(
    State(state): State<Shared<DiscoveryState>>,
    RawQuery(query): RawQuery,
    headers: HeaderMap,
) -> Response {
    observe_request(&state, query.as_deref(), &headers);
    match state.course_mode.load(Ordering::SeqCst) {
        REST_REJECT_DASHBOARD_SUCCESS | REST_REJECT_DASHBOARD_EMPTY => {
            return (StatusCode::UNAUTHORIZED, "authorization required").into_response();
        }
        REST_CSRF_REQUIRED => {
            return (StatusCode::FORBIDDEN, "CSRF verification failed").into_response();
        }
        REST_PAT_REQUIRED => {
            return (StatusCode::FORBIDDEN, "personal access token required").into_response();
        }
        REST_MALFORMED => {
            return ([("content-type", "application/json")], "{not-json").into_response();
        }
        REST_OPTIONAL_FIELDS_MISSING => {
            return (
                [("content-type", "application/json")],
                r#"[{"id":101,"name":"Named course"},{"id":102}]"#,
            )
                .into_response();
        }
        REST_SUCCESS => {}
        _ => panic!("unknown test course mode"),
    }
    (
        [("content-type", "application/json")],
        r#"[{"id":101,"name":"Course A","course_code":"A-101","term":{"name":"Term A"}}]"#,
    )
        .into_response()
}

async fn canvas_dashboard(State(state): State<Shared<DiscoveryState>>) -> Html<&'static str> {
    if state.course_mode.load(Ordering::SeqCst) == REST_REJECT_DASHBOARD_EMPTY {
        return Html("<!doctype html><main>No bootstrap data</main>");
    }
    Html(
        r#"<!doctype html><script id="dashboard_cards" type="application/json">[{"id":202,"name":"Course B","course_code":"B-202","term":{"name":"Term B"}}]</script>"#,
    )
}

fn observe_request(state: &DiscoveryState, query: Option<&str>, headers: &HeaderMap) {
    state
        .saw_authorization
        .store(headers.contains_key("authorization"), Ordering::SeqCst);
    let has_query = query.is_some_and(|value| {
        value.matches("include%5B%5D=").count() == 2 && value.contains("per_page=100")
    });
    state.saw_course_query.store(has_query, Ordering::SeqCst);
}
