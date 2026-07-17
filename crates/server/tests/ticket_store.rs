use std::{sync::Arc, time::Duration};

use canvas_core::{
    canvas::{CanvasCourse, IdentitySource, UserIdentity},
    client::{DnsOverride, ProtocolConfig, ProtocolContext},
    video::{CanvasVideo, VideoTrack, VideoTrackInput, VideoTrackKind},
};
use secrecy::SecretString;
use server::{
    session::{
        CourseHandle, HandleWindow, TrackHandle, TrackParent, TrackRegistration, UserSession,
        UserSessionOptions, VideoHandle,
    },
    ticket::{DownloadTicketStore, TicketLookupError, TicketRequest},
};
use time::OffsetDateTime;
use url::Url;

fn context() -> ProtocolContext {
    let origin = Url::parse("http://content.mock.test:9/").expect("origin should parse");
    let address = "127.0.0.1:9".parse().expect("address should parse");
    let config = ProtocolConfig::mock(origin, Duration::from_secs(1))
        .with_dns_overrides(vec![DnsOverride::new("content.mock.test", address)]);
    ProtocolContext::new(config).expect("context should build")
}

fn session(stable_id: &str) -> Arc<UserSession> {
    Arc::new(
        UserSession::new(
            UserIdentity {
                stable_id: SecretString::from(stable_id.to_owned()),
                account: None,
                display_name: None,
                source: IdentitySource::MySjtuAccount,
            },
            context(),
            UserSessionOptions {
                expires_at: OffsetDateTime::now_utc() + time::Duration::hours(1),
                max_downloads: 1,
            },
        )
        .expect("session should build"),
    )
}

async fn ticket_request(session: &Arc<UserSession>, now: OffsetDateTime) -> TicketRequest {
    let window = HandleWindow::new(now, time::Duration::minutes(5));
    let course = register_course(session, window).await;
    let video = register_video(session, &course, window).await;
    let parent = TrackParent { course, video };
    let track = register_track(session, parent.clone(), window).await;
    let record = session
        .resources()
        .resolve_track(&parent, &track, now)
        .await
        .expect("track should resolve");
    TicketRequest {
        session_id: session.id().clone(),
        parent,
        track,
        record,
    }
}

async fn register_course(session: &UserSession, window: HandleWindow) -> CourseHandle {
    session
        .resources()
        .replace_courses(
            vec![CanvasCourse {
                id: 1,
                name: "Course".to_owned(),
                course_code: String::new(),
                term: None,
            }],
            window,
        )
        .await
        .expect("course handle")[0]
        .handle
        .clone()
}

async fn register_video(
    session: &UserSession,
    course: &CourseHandle,
    window: HandleWindow,
) -> VideoHandle {
    let video = CanvasVideo {
        id: "video".to_owned(),
        name: "Recording".to_owned(),
        started_at: None,
        ended_at: None,
    };
    session
        .resources()
        .replace_videos(course, vec![video], window)
        .await
        .expect("video handle")[0]
        .handle
        .clone()
}

async fn register_track(
    session: &UserSession,
    parent: TrackParent,
    window: HandleWindow,
) -> TrackHandle {
    let track = VideoTrack::new(VideoTrackInput {
        id: "track".to_owned(),
        kind: VideoTrackKind::Screen,
        suggested_filename: "../recording\r\n.mp4".to_owned(),
        upstream_url: SecretString::from(
            "http://content.mock.test:9/video.mp4?token=URL_CANARY".to_owned(),
        ),
    });
    let registration = TrackRegistration::from_track(session.protocol(), track)
        .await
        .expect("resource should validate");
    session
        .resources()
        .replace_tracks(parent, vec![registration], window)
        .await
        .expect("track handle")[0]
        .handle
        .clone()
}

#[tokio::test]
async fn ticket_is_reusable_only_by_its_session_until_expiry() {
    let now = OffsetDateTime::now_utc();
    let first = session("first");
    let second = session("second");
    let store = DownloadTicketStore::new();
    let request = ticket_request(&first, now).await;
    let ticket = store
        .issue(request, now, time::Duration::seconds(60))
        .expect("ticket should be issued");

    assert!(matches!(
        store.resolve(ticket.id(), second.id(), now),
        Err(TicketLookupError::SessionMismatch)
    ));
    let first_use = store
        .resolve(ticket.id(), first.id(), now)
        .expect("owner should resolve ticket");
    let second_use = store
        .resolve(ticket.id(), first.id(), now + time::Duration::seconds(30))
        .expect("ticket should support another Range request");
    assert!(Arc::ptr_eq(&first_use, &second_use));
    assert!(!format!("{ticket:?}").contains("URL_CANARY"));

    assert!(matches!(
        store.resolve(ticket.id(), first.id(), now + time::Duration::seconds(61)),
        Err(TicketLookupError::Expired)
    ));
}
