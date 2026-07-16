#[path = "support/lti_fixture.rs"]
mod lti_fixture;
mod support;

use std::sync::{Arc, atomic::Ordering};

use canvas_core::{
    client::ProtocolContext,
    error::ProtocolError,
    lti::establish_course_video_session,
    video::{
        VideoTrackKind, get_video_info, list_course_videos, list_course_videos_with_refresh,
        probe_video_track,
    },
};
use secrecy::ExposeSecret;

use lti_fixture::{
    CANVAS_COURSE_ID, FlowState, VIDEO_COURSE_ID, VIDEO_TOKEN, VIDEO_TOKEN_B, router,
};
use support::{MockServer, topology::MockTopology};

#[tokio::test]
async fn mock_lti_video_detail_and_range_chain_uses_course_bound_auth() {
    let state = Arc::new(FlowState::default());
    let server = MockServer::spawn(router(state.clone())).await;
    let topology = MockTopology::for_server(&server);
    let context = ProtocolContext::new(topology.config).expect("context should build");

    let auth = establish_course_video_session(&context, CANVAS_COURSE_ID)
        .await
        .expect("mock LTI launch should succeed");
    assert_eq!(auth.canvas_course_id, CANVAS_COURSE_ID);
    assert_eq!(auth.video_course_id, VIDEO_COURSE_ID);
    assert_eq!(auth.token.expose_secret(), VIDEO_TOKEN);
    assert!(!format!("{auth:?}").contains(VIDEO_TOKEN));

    let videos = list_course_videos(&context, &auth)
        .await
        .expect("video list should parse");
    let info = get_video_info(&context, &auth, &videos[0].id)
        .await
        .expect("video detail should parse");
    assert_eq!(info.tracks.len(), 2);
    assert_eq!(info.tracks[0].kind, VideoTrackKind::Screen);
    assert!(!format!("{:?}", info.tracks).contains("content.sjtu.mock.test"));

    let probe = probe_video_track(&context, &info.tracks[0])
        .await
        .expect("Range probe should succeed");
    assert_eq!((probe.status, probe.total_size), (206, Some(4096)));
    assert!(probe.supports_range);
    assert_eq!(state.lti_launches.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn expired_video_token_relaunches_lti_exactly_once() {
    let state = Arc::new(FlowState::default());
    state.stale_first_token.store(true, Ordering::SeqCst);
    let server = MockServer::spawn(router(state.clone())).await;
    let context = ProtocolContext::new(MockTopology::for_server(&server).config)
        .expect("context should build");

    let catalog = list_course_videos_with_refresh(&context, CANVAS_COURSE_ID)
        .await
        .expect("one explicit refresh should recover");

    assert_eq!(catalog.videos.len(), 1);
    assert_eq!(state.token_exchanges.load(Ordering::SeqCst), 2);
    assert_eq!(state.list_calls.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn a_second_expired_token_is_not_retried_again() {
    let state = Arc::new(FlowState::default());
    state.always_stale.store(true, Ordering::SeqCst);
    let server = MockServer::spawn(router(state.clone())).await;
    let context = ProtocolContext::new(MockTopology::for_server(&server).config)
        .expect("context should build");

    let error = list_course_videos_with_refresh(&context, CANVAS_COURSE_ID)
        .await
        .expect_err("a second token failure must surface");

    assert_eq!(error, ProtocolError::VideoTokenExpired);
    assert_eq!(state.token_exchanges.load(Ordering::SeqCst), 2);
    assert_eq!(state.list_calls.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn lti_redirect_to_video_content_origin_is_rejected() {
    let state = Arc::new(FlowState::default());
    state.malicious_redirect.store(true, Ordering::SeqCst);
    let server = MockServer::spawn(router(state)).await;
    let context = ProtocolContext::new(MockTopology::for_server(&server).config)
        .expect("context should build");

    let error = establish_course_video_session(&context, CANVAS_COURSE_ID)
        .await
        .expect_err("cross-purpose LTI redirect must fail");

    assert_eq!(error, ProtocolError::LtiRedirectInvalid);
}

#[tokio::test]
async fn token_exchange_missing_data_is_not_reported_as_success() {
    let state = Arc::new(FlowState::default());
    state.missing_token_data.store(true, Ordering::SeqCst);
    let server = MockServer::spawn(router(state)).await;
    let context = ProtocolContext::new(MockTopology::for_server(&server).config)
        .expect("context should build");

    let error = establish_course_video_session(&context, CANVAS_COURSE_ID)
        .await
        .expect_err("missing token fields must fail");

    assert_eq!(error, ProtocolError::VideoTokenExchangeFailed);
}

#[tokio::test]
async fn two_contexts_keep_cookie_stores_and_course_tokens_isolated() {
    let state_a = Arc::new(FlowState::default());
    let state_b = Arc::new(FlowState::default());
    state_b.alternate_token.store(true, Ordering::SeqCst);
    let server_a = MockServer::spawn(router(state_a)).await;
    let server_b = MockServer::spawn(router(state_b)).await;
    let context_a = ProtocolContext::new(MockTopology::for_server(&server_a).config)
        .expect("context A should build");
    let context_b = ProtocolContext::new(MockTopology::for_server(&server_b).config)
        .expect("context B should build");

    let auth_a = establish_course_video_session(&context_a, CANVAS_COURSE_ID)
        .await
        .expect("context A LTI should succeed");
    let auth_b = establish_course_video_session(&context_b, CANVAS_COURSE_ID)
        .await
        .expect("context B LTI should succeed");

    assert_eq!(auth_a.token.expose_secret(), VIDEO_TOKEN);
    assert_eq!(auth_b.token.expose_secret(), VIDEO_TOKEN_B);
    assert!(!Arc::ptr_eq(
        &context_a.cookie_store,
        &context_b.cookie_store
    ));
    assert!(list_course_videos(&context_a, &auth_a).await.is_ok());
    assert!(list_course_videos(&context_b, &auth_b).await.is_ok());
    assert!(list_course_videos(&context_a, &auth_a).await.is_ok());
}
