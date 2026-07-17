use std::{collections::HashMap, sync::Arc};

use canvas_core::{canvas::CanvasCourse, lti::CourseVideoAuth, video::CanvasVideo};
use thiserror::Error;
use time::OffsetDateTime;
use tokio::sync::Mutex;

use super::{
    CourseHandle, CourseRecord, CourseView, HandleWindow, TrackHandle, TrackParent, TrackRecord,
    TrackRegistration, TrackView, VideoHandle, VideoRecord, VideoView,
};
use crate::id::RandomIdError;

#[derive(Default)]
struct ResourceData {
    courses: HashMap<CourseHandle, CourseRecord>,
    videos: HashMap<VideoHandle, VideoRecord>,
    tracks: HashMap<TrackHandle, TrackRecord>,
    course_auth: HashMap<CourseHandle, Arc<CourseVideoAuth>>,
}

#[derive(Default)]
pub struct SessionResources {
    data: Mutex<ResourceData>,
}

#[derive(Debug, Error)]
pub enum HandleError {
    #[error(transparent)]
    Random(#[from] RandomIdError),
    #[error("parent handle is invalid")]
    ParentInvalid,
}

impl SessionResources {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn replace_courses(
        &self,
        courses: Vec<CanvasCourse>,
        window: HandleWindow,
    ) -> Result<Vec<CourseView>, HandleError> {
        let mut records = HashMap::new();
        let mut views = Vec::with_capacity(courses.len());
        for course in courses {
            let handle = CourseHandle::generate()?;
            records.insert(
                handle.clone(),
                CourseRecord {
                    canvas_id: course.id,
                    expires_at: window.expires_at,
                },
            );
            views.push(course_view(handle, course));
        }
        let mut data = self.data.lock().await;
        data.courses = records;
        data.videos.clear();
        data.tracks.clear();
        data.course_auth.clear();
        Ok(views)
    }

    pub async fn resolve_course(
        &self,
        handle: &CourseHandle,
        now: OffsetDateTime,
    ) -> Option<CourseRecord> {
        let data = self.data.lock().await;
        data.courses
            .get(handle)
            .filter(|record| record.expires_at > now)
            .cloned()
    }

    pub async fn replace_videos(
        &self,
        course: &CourseHandle,
        videos: Vec<CanvasVideo>,
        window: HandleWindow,
    ) -> Result<Vec<VideoView>, HandleError> {
        let mut data = self.data.lock().await;
        let course_valid = data
            .courses
            .get(course)
            .is_some_and(|record| record.expires_at > window.now);
        if !course_valid {
            return Err(HandleError::ParentInvalid);
        }
        data.videos.retain(|_, record| &record.course != course);
        data.tracks.retain(|_, record| &record.course != course);
        let mut views = Vec::with_capacity(videos.len());
        for video in videos {
            let handle = VideoHandle::generate()?;
            data.videos.insert(
                handle.clone(),
                VideoRecord {
                    real_id: video.id,
                    course: course.clone(),
                    expires_at: window.expires_at,
                },
            );
            views.push(VideoView {
                handle,
                name: video.name,
                started_at: video.started_at,
            });
        }
        Ok(views)
    }

    pub async fn resolve_video(
        &self,
        course: &CourseHandle,
        video: &VideoHandle,
        now: OffsetDateTime,
    ) -> Option<VideoRecord> {
        let data = self.data.lock().await;
        data.videos
            .get(video)
            .filter(|record| &record.course == course && record.expires_at > now)
            .cloned()
    }

    pub async fn set_course_auth(&self, course: CourseHandle, auth: Arc<CourseVideoAuth>) {
        self.data.lock().await.course_auth.insert(course, auth);
    }

    pub async fn course_auth(&self, course: &CourseHandle) -> Option<Arc<CourseVideoAuth>> {
        self.data.lock().await.course_auth.get(course).cloned()
    }

    pub async fn replace_tracks(
        &self,
        parent: TrackParent,
        tracks: Vec<TrackRegistration>,
        window: HandleWindow,
    ) -> Result<Vec<TrackView>, HandleError> {
        let mut data = self.data.lock().await;
        if !valid_video_parent(&data, &parent, window.now) {
            return Err(HandleError::ParentInvalid);
        }
        data.tracks.retain(|_, record| record.video != parent.video);
        let mut views = Vec::with_capacity(tracks.len());
        for track in tracks {
            let handle = TrackHandle::generate()?;
            data.tracks.insert(
                handle.clone(),
                TrackRecord {
                    course: parent.course.clone(),
                    video: parent.video.clone(),
                    resource: track.resource,
                    suggested_filename: track.suggested_filename.clone(),
                    expires_at: window.expires_at,
                },
            );
            views.push(TrackView {
                handle,
                kind: track.kind,
                suggested_filename: track.suggested_filename,
            });
        }
        Ok(views)
    }

    pub async fn resolve_track(
        &self,
        parent: &TrackParent,
        track: &TrackHandle,
        now: OffsetDateTime,
    ) -> Option<TrackRecord> {
        let data = self.data.lock().await;
        data.tracks
            .get(track)
            .filter(|record| {
                record.course == parent.course
                    && record.video == parent.video
                    && record.expires_at > now
            })
            .cloned()
    }
}

fn course_view(handle: CourseHandle, course: CanvasCourse) -> CourseView {
    CourseView {
        handle,
        name: course.name,
        course_code: (!course.course_code.is_empty()).then_some(course.course_code),
        term_name: course.term.map(|term| term.name),
    }
}

fn valid_video_parent(data: &ResourceData, parent: &TrackParent, now: OffsetDateTime) -> bool {
    data.videos
        .get(&parent.video)
        .is_some_and(|record| record.course == parent.course && record.expires_at > now)
}
