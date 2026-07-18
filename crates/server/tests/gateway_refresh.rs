#[path = "../../canvas-core/tests/support/lti_fixture.rs"]
mod lti_fixture;
#[path = "../../canvas-core/tests/support/mod.rs"]
mod support;

use std::sync::{Arc, atomic::Ordering};

use canvas_core::{ProtocolError, client::ProtocolContext, lti::CourseVideoAuth};
use secrecy::SecretString;
use server::gateway::{ProductionProtocolGateway, ProtocolGateway, VideoDetailRequest};

use lti_fixture::{CANVAS_COURSE_ID, FlowState, VIDEO_COURSE_ID, router};
use support::{MockServer, topology::MockTopology};

#[tokio::test]
async fn expired_detail_token_relaunches_lti_exactly_once() {
    let state = Arc::new(FlowState::default());
    let server = MockServer::spawn(router(state.clone())).await;
    let context =
        ProtocolContext::new(MockTopology::for_server(&server).config).expect("protocol context");
    let detail = ProductionProtocolGateway
        .video_detail(
            &context,
            VideoDetailRequest {
                canvas_course_id: CANVAS_COURSE_ID,
                auth: Some(stale_auth()),
                video_id: "video-abc",
            },
        )
        .await
        .expect("one refresh should recover");

    assert_eq!(detail.info.tracks.len(), 2);
    assert_eq!(state.lti_launches.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn second_expired_detail_token_is_not_retried_again() {
    let state = Arc::new(FlowState::default());
    state.always_stale_detail.store(true, Ordering::SeqCst);
    let server = MockServer::spawn(router(state.clone())).await;
    let context =
        ProtocolContext::new(MockTopology::for_server(&server).config).expect("protocol context");
    let error = ProductionProtocolGateway
        .video_detail(
            &context,
            VideoDetailRequest {
                canvas_course_id: CANVAS_COURSE_ID,
                auth: Some(stale_auth()),
                video_id: "video-abc",
            },
        )
        .await
        .err()
        .expect("second token expiry must surface");

    assert_eq!(error, ProtocolError::VideoTokenExpired);
    assert_eq!(state.lti_launches.load(Ordering::SeqCst), 1);
    assert_eq!(state.detail_calls.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn expired_subtitle_authorization_relaunches_lti_exactly_once() {
    let state = Arc::new(FlowState::default());
    let server = MockServer::spawn(router(state.clone())).await;
    let context =
        ProtocolContext::new(MockTopology::for_server(&server).config).expect("protocol context");
    let subtitle = ProductionProtocolGateway
        .subtitle(
            &context,
            VideoDetailRequest {
                canvas_course_id: CANVAS_COURSE_ID,
                auth: Some(stale_auth()),
                video_id: "video-abc",
            },
        )
        .await
        .expect("one refresh should recover subtitle access");

    assert!(subtitle.document.srt.contains("第一句"));
    assert_eq!(state.lti_launches.load(Ordering::SeqCst), 1);
    assert_eq!(state.detail_calls.load(Ordering::SeqCst), 2);
    assert_eq!(state.subtitle_calls.load(Ordering::SeqCst), 1);
}

fn stale_auth() -> Arc<CourseVideoAuth> {
    Arc::new(CourseVideoAuth {
        canvas_course_id: CANVAS_COURSE_ID,
        video_course_id: VIDEO_COURSE_ID.to_owned(),
        token: SecretString::from("stale-video-token".to_owned()),
    })
}
