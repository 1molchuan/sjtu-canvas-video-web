mod support;

use canvas_core::{
    client::ProtocolContext,
    video::{ValidatedUpstreamResource, VideoTrack, VideoTrackInput, VideoTrackKind},
};
use secrecy::SecretString;

use support::{MockServer, topology::MockTopology};

#[tokio::test]
async fn validated_video_resource_is_resolved_without_debug_disclosure() {
    const QUERY_CANARY: &str = "UPSTREAM_QUERY_SECRET_CANARY";
    let server = MockServer::spawn(axum::Router::new()).await;
    let topology = MockTopology::for_server(&server);
    let context = ProtocolContext::new(topology.config).expect("context should build");
    let upstream = topology
        .video_content_origin
        .join(&format!("recording.mp4?token={QUERY_CANARY}"))
        .expect("fixture URL should build");
    let track = VideoTrack::new(VideoTrackInput {
        id: "track-1".to_owned(),
        kind: VideoTrackKind::Screen,
        suggested_filename: "recording.mp4".to_owned(),
        upstream_url: SecretString::from(upstream.to_string()),
    });

    let resource = ValidatedUpstreamResource::from_track(&context, &track)
        .await
        .expect("allowlisted mock resource should validate");
    let resolved = resource
        .validated_url(&context)
        .await
        .expect("resource should revalidate");

    assert_eq!(
        resolved.host_str(),
        topology.video_content_origin.host_str()
    );
    assert!(resolved.query().is_some());
    assert!(!format!("{resource:?}").contains(QUERY_CANARY));
}
