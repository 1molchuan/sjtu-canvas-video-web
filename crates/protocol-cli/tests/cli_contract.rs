use clap::Parser;
use protocol_cli::{
    args::{Cli, Command},
    gate::real_protocol_enabled,
    report::{GoNoGoDecision, StepName, StepStatus, ValidationReport},
};

#[test]
fn full_command_accepts_required_phase_one_flags() {
    let cli = Cli::try_parse_from([
        "protocol-cli",
        "--debug",
        "--json-output",
        "--timeout-seconds",
        "120",
        "--no-course-discovery",
        "full",
        "--course-id",
        "42",
        "--video-id",
        "video-1",
        "--probe-direct",
    ])
    .expect("valid CLI should parse");

    assert!(cli.debug);
    assert!(cli.json_output);
    assert_eq!(cli.timeout_seconds, 120);
    assert!(cli.no_course_discovery);
    let Command::Full(args) = cli.command else {
        panic!("expected full command");
    };
    assert_eq!(args.course_id, 42);
    assert_eq!(args.video_id.as_deref(), Some("video-1"));
    assert!(args.probe_direct);
}

#[test]
fn report_distinguishes_go_a_go_b_no_go_and_undetermined() {
    let mut report = ValidationReport::new("2026-07-17T00:00:00Z".to_owned());
    mark_video_chain_passed(&mut report);
    report.set_step(StepName::CourseDiscovery, StepStatus::CookieSessionRejected);
    report.finalize_decision();
    assert_eq!(report.decision, GoNoGoDecision::GoB);

    report.set_step(StepName::CourseDiscovery, StepStatus::Passed);
    report.finalize_decision();
    assert_eq!(report.decision, GoNoGoDecision::GoA);

    report.set_step(StepName::CanvasLogin, StepStatus::Failed);
    report.finalize_decision();
    assert_eq!(report.decision, GoNoGoDecision::NoGoC);
}

fn mark_video_chain_passed(report: &mut ValidationReport) {
    for step in [
        StepName::JAccountUuid,
        StepName::JAccountWebsocket,
        StepName::QrCode,
        StepName::ExpressLogin,
        StepName::CanvasLogin,
        StepName::LtiLaunch,
        StepName::VideoList,
        StepName::VideoDetail,
        StepName::RangeProbe,
    ] {
        report.set_step(step, StepStatus::Passed);
    }
}

#[test]
fn inspect_course_rejects_non_positive_course_id() {
    let result = Cli::try_parse_from(["protocol-cli", "inspect-course", "--course-id", "0"]);

    assert!(result.is_err());
}

#[test]
fn real_mode_requires_the_exact_explicit_value() {
    assert!(real_protocol_enabled(Some("1")));
    assert!(!real_protocol_enabled(None));
    assert!(!real_protocol_enabled(Some("true")));
    assert!(!real_protocol_enabled(Some("0")));
}

#[test]
fn report_json_contains_statuses_but_not_secret_canaries() {
    let mut report = ValidationReport::new("2026-07-17T00:00:00Z".to_owned());
    report.set_step(StepName::JAccountUuid, StepStatus::Passed);
    report.set_step(StepName::LtiLaunch, StepStatus::Failed);
    report.set_video_metadata("live.sjtu.edu.cn".to_owned(), true);

    let json = serde_json::to_string(&report).expect("report should serialize");

    assert!(json.contains("jaccount_uuid"));
    assert!(json.contains("live.sjtu.edu.cn"));
    for secret in [
        "JAAuthCookie",
        "tokenId",
        "video-token-secret",
        "https://live",
    ] {
        assert!(!json.contains(secret));
    }
}
