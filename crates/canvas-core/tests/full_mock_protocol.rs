#[path = "support/full_fixture.rs"]
mod full_fixture;
#[path = "support/lti_fixture.rs"]
mod lti_fixture;
mod support;

use std::{sync::Arc, time::Duration};

use canvas_core::{
    canvas::{CourseDiscoveryOutcome, discover_courses, establish_canvas_session, probe_identity},
    client::ProtocolContext,
    jaccount::{QrLoginOptions, login_with_qr},
    video::{get_video_info, list_course_videos_with_refresh, probe_video_track},
};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use lti_fixture::{CANVAS_COURSE_ID, FlowState};
use support::{MockServer, topology::MockTopology};

#[tokio::test]
async fn complete_mock_protocol_chain_reuses_one_isolated_context() {
    let flow_state = Arc::new(FlowState::default());
    let server = MockServer::spawn(full_fixture::router(flow_state)).await;
    let context = ProtocolContext::new(MockTopology::with_socket_jaccount(&server).config)
        .expect("isolated protocol context should build");
    let (progress, _receiver) = mpsc::unbounded_channel();

    let jaccount = login_with_qr(
        &context,
        QrLoginOptions {
            timeout: Duration::from_secs(2),
            refresh_interval: Duration::from_secs(1),
            cancellation: CancellationToken::new(),
        },
        progress,
    )
    .await
    .expect("jAccount mock login should succeed");
    establish_canvas_session(&context, &jaccount.ja_auth_cookie)
        .await
        .expect("Canvas mock SSO should succeed");
    probe_identity(&context)
        .await
        .expect("stable mock identity should be found");
    let courses = discover_courses(&context)
        .await
        .expect("course discovery experiment should complete");
    let CourseDiscoveryOutcome::Success { courses, .. } = courses else {
        panic!("Cookie REST course discovery should succeed");
    };
    assert_eq!(courses[0].id, CANVAS_COURSE_ID);

    let catalog = list_course_videos_with_refresh(&context, CANVAS_COURSE_ID)
        .await
        .expect("LTI and video list should succeed");
    let detail = get_video_info(&context, &catalog.auth, &catalog.videos[0].id)
        .await
        .expect("video detail should succeed");
    let probe = probe_video_track(&context, &detail.tracks[0])
        .await
        .expect("Range probe should succeed");

    assert_eq!(probe.status, 206);
    assert_eq!(probe.total_size, Some(4096));
}
