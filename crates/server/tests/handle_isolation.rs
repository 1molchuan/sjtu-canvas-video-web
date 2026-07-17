use std::{net::SocketAddr, sync::Arc, time::Duration};

use canvas_core::{
    canvas::{CanvasCourse, IdentitySource, UserIdentity},
    client::{DnsOverride, ProtocolConfig, ProtocolContext},
    video::{CanvasVideo, VideoTrack, VideoTrackInput, VideoTrackKind},
};
use secrecy::SecretString;
use server::session::{
    CourseHandle, HandleWindow, TrackParent, TrackRegistration, UserSession, UserSessionOptions,
    VideoHandle,
};
use time::OffsetDateTime;
use url::Url;

const CONTENT_HOST: &str = "content.example.test";

struct CourseFixture {
    first: Arc<UserSession>,
    second: Arc<UserSession>,
    course_a: CourseHandle,
    course_b: CourseHandle,
    now: OffsetDateTime,
    window: HandleWindow,
}

#[tokio::test]
async fn course_handles_are_bound_to_one_session() {
    let fixture = course_fixture().await;
    let own = fixture
        .first
        .resources()
        .resolve_course(&fixture.course_a, fixture.now)
        .await;
    assert_eq!(own.expect("own course handle").canvas_id, 11);
    assert!(
        fixture
            .second
            .resources()
            .resolve_course(&fixture.course_a, fixture.now)
            .await
            .is_none()
    );
}

#[tokio::test]
async fn video_handles_are_bound_to_their_course_and_session() {
    let fixture = course_fixture().await;
    let video = register_videos(&fixture, &["video-a"]).await[0].clone();
    assert!(resolve_video(&fixture, &fixture.course_a, &video).await);
    assert!(!resolve_video(&fixture, &fixture.course_b, &video).await);
    assert!(
        fixture
            .second
            .resources()
            .resolve_video(&fixture.course_a, &video, fixture.now)
            .await
            .is_none()
    );
}

#[tokio::test]
async fn track_handles_are_bound_to_their_video_course_and_session() {
    let fixture = course_fixture().await;
    let videos = register_videos(&fixture, &["video-a", "video-b"]).await;
    let first_video = videos[0].clone();
    let second_video = videos[1].clone();
    let parent = TrackParent {
        course: fixture.course_a.clone(),
        video: first_video.clone(),
    };
    let track = register_track(&fixture, parent.clone()).await;
    assert!(
        fixture
            .first
            .resources()
            .resolve_track(&parent, &track, fixture.now)
            .await
            .is_some()
    );
    let wrong_video = TrackParent {
        course: fixture.course_a.clone(),
        video: second_video,
    };
    assert!(
        fixture
            .first
            .resources()
            .resolve_track(&wrong_video, &track, fixture.now)
            .await
            .is_none()
    );
    assert!(
        fixture
            .second
            .resources()
            .resolve_track(&parent, &track, fixture.now)
            .await
            .is_none()
    );
}

#[tokio::test]
async fn expired_handles_are_not_resolved() {
    let fixture = course_fixture().await;
    let expired_at = fixture.now + time::Duration::minutes(6);
    assert!(
        fixture
            .first
            .resources()
            .resolve_course(&fixture.course_a, expired_at)
            .await
            .is_none()
    );
}

async fn course_fixture() -> CourseFixture {
    let now = OffsetDateTime::now_utc();
    let window = HandleWindow::new(now, time::Duration::minutes(5));
    let first = session("first");
    let views = first
        .resources()
        .replace_courses(courses(), window)
        .await
        .expect("course handles");
    CourseFixture {
        first,
        second: session("second"),
        course_a: views[0].handle.clone(),
        course_b: views[1].handle.clone(),
        now,
        window,
    }
}

fn session(stable_id: &str) -> Arc<UserSession> {
    let identity = UserIdentity {
        stable_id: SecretString::from(stable_id.to_owned()),
        account: None,
        display_name: None,
        source: IdentitySource::MySjtuAccount,
    };
    Arc::new(
        UserSession::new(
            identity,
            protocol(),
            UserSessionOptions {
                expires_at: OffsetDateTime::now_utc() + time::Duration::hours(1),
                max_downloads: 1,
            },
        )
        .expect("session"),
    )
}

fn protocol() -> ProtocolContext {
    let origin = Url::parse("http://content.example.test:9/").expect("origin");
    let address: SocketAddr = "127.0.0.1:9".parse().expect("address");
    let config = ProtocolConfig::mock(origin, Duration::from_secs(1))
        .with_dns_overrides(vec![DnsOverride::new(CONTENT_HOST, address)]);
    ProtocolContext::new(config).expect("context")
}

fn courses() -> Vec<CanvasCourse> {
    [(11, "Course A"), (22, "Course B")]
        .into_iter()
        .map(|(id, name)| CanvasCourse {
            id,
            name: name.to_owned(),
            course_code: String::new(),
            term: None,
        })
        .collect()
}

async fn register_videos(fixture: &CourseFixture, ids: &[&str]) -> Vec<VideoHandle> {
    let videos = ids
        .iter()
        .map(|id| CanvasVideo {
            id: (*id).to_owned(),
            name: "Recording".to_owned(),
            started_at: None,
            ended_at: None,
        })
        .collect();
    fixture
        .first
        .resources()
        .replace_videos(&fixture.course_a, videos, fixture.window)
        .await
        .expect("video handles")
        .into_iter()
        .map(|view| view.handle)
        .collect()
}

async fn register_track(
    fixture: &CourseFixture,
    parent: TrackParent,
) -> server::session::TrackHandle {
    let input = VideoTrackInput {
        id: "real-track".to_owned(),
        kind: VideoTrackKind::Screen,
        suggested_filename: "recording.mp4".to_owned(),
        upstream_url: SecretString::from("http://content.example.test:9/video.mp4".to_owned()),
    };
    let registration =
        TrackRegistration::from_track(fixture.first.protocol(), VideoTrack::new(input))
            .await
            .expect("validated track");
    fixture
        .first
        .resources()
        .replace_tracks(parent, vec![registration], fixture.window)
        .await
        .expect("track handle")[0]
        .handle
        .clone()
}

async fn resolve_video(
    fixture: &CourseFixture,
    course: &CourseHandle,
    video: &VideoHandle,
) -> bool {
    fixture
        .first
        .resources()
        .resolve_video(course, video, fixture.now)
        .await
        .is_some()
}
