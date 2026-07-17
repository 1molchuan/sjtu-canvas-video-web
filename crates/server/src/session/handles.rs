use canvas_core::{
    client::ProtocolContext,
    video::{ValidatedUpstreamResource, VideoTrack, VideoTrackKind},
};
use time::OffsetDateTime;

use crate::id::opaque_id;

opaque_id!(CourseHandle);
opaque_id!(VideoHandle);
opaque_id!(TrackHandle);

#[derive(Clone, Copy)]
pub struct HandleWindow {
    pub now: OffsetDateTime,
    pub expires_at: OffsetDateTime,
}

impl HandleWindow {
    pub fn new(now: OffsetDateTime, ttl: time::Duration) -> Self {
        Self {
            now,
            expires_at: now + ttl,
        }
    }
}

#[derive(Clone)]
pub struct CourseView {
    pub handle: CourseHandle,
    pub name: String,
    pub course_code: Option<String>,
    pub term_name: Option<String>,
}

#[derive(Clone)]
pub struct CourseRecord {
    pub canvas_id: i64,
    pub expires_at: OffsetDateTime,
}

#[derive(Clone)]
pub struct VideoView {
    pub handle: VideoHandle,
    pub name: String,
    pub started_at: Option<String>,
}

#[derive(Clone)]
pub struct VideoRecord {
    pub real_id: String,
    pub course: CourseHandle,
    pub expires_at: OffsetDateTime,
}

#[derive(Clone)]
pub struct TrackView {
    pub handle: TrackHandle,
    pub kind: VideoTrackKind,
    pub suggested_filename: String,
}

#[derive(Clone)]
pub struct TrackRecord {
    pub course: CourseHandle,
    pub video: VideoHandle,
    pub resource: ValidatedUpstreamResource,
    pub suggested_filename: String,
    pub expires_at: OffsetDateTime,
}

pub struct TrackRegistration {
    pub kind: VideoTrackKind,
    pub suggested_filename: String,
    pub resource: ValidatedUpstreamResource,
}

#[derive(Clone)]
pub struct TrackParent {
    pub course: CourseHandle,
    pub video: VideoHandle,
}

impl TrackRegistration {
    pub async fn from_track(
        context: &ProtocolContext,
        track: VideoTrack,
    ) -> Result<Self, canvas_core::ProtocolError> {
        let resource = ValidatedUpstreamResource::from_track(context, &track).await?;
        Ok(Self {
            kind: track.kind,
            suggested_filename: track.suggested_filename,
            resource,
        })
    }
}
