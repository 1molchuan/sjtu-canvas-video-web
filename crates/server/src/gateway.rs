use std::sync::Arc;

use async_trait::async_trait;
use canvas_core::{
    ProtocolError,
    canvas::{CanvasCourse, CourseDiscoveryOutcome, discover_courses},
    client::ProtocolContext,
    lti::{CourseVideoAuth, establish_course_video_session},
    video::{CanvasVideo, VideoInfo, get_video_info, list_course_videos_with_refresh},
};

pub struct VideoDetailSession {
    pub auth: Arc<CourseVideoAuth>,
    pub info: VideoInfo,
}

pub struct VideoDetailRequest<'a> {
    pub canvas_course_id: i64,
    pub auth: Option<Arc<CourseVideoAuth>>,
    pub video_id: &'a str,
}

#[async_trait]
pub trait ProtocolGateway: Send + Sync {
    async fn courses(&self, context: &ProtocolContext) -> Result<Vec<CanvasCourse>, ProtocolError>;

    async fn videos(
        &self,
        context: &ProtocolContext,
        canvas_course_id: i64,
    ) -> Result<(CourseVideoAuth, Vec<CanvasVideo>), ProtocolError>;

    async fn video_detail(
        &self,
        context: &ProtocolContext,
        request: VideoDetailRequest<'_>,
    ) -> Result<VideoDetailSession, ProtocolError>;
}

pub struct ProductionProtocolGateway;

#[async_trait]
impl ProtocolGateway for ProductionProtocolGateway {
    async fn courses(&self, context: &ProtocolContext) -> Result<Vec<CanvasCourse>, ProtocolError> {
        match discover_courses(context).await? {
            CourseDiscoveryOutcome::Success { courses, .. } => Ok(courses),
            CourseDiscoveryOutcome::RequiresPersonalAccessToken
            | CourseDiscoveryOutcome::CookieSessionRejected
            | CourseDiscoveryOutcome::CsrfRequired => {
                Err(ProtocolError::CanvasCourseDiscoveryRejected)
            }
            CourseDiscoveryOutcome::UnsupportedResponse
            | CourseDiscoveryOutcome::UpstreamChanged => {
                Err(ProtocolError::CanvasCourseDiscoveryUnsupported)
            }
        }
    }

    async fn videos(
        &self,
        context: &ProtocolContext,
        canvas_course_id: i64,
    ) -> Result<(CourseVideoAuth, Vec<CanvasVideo>), ProtocolError> {
        let session = list_course_videos_with_refresh(context, canvas_course_id).await?;
        Ok((session.auth, session.videos))
    }

    async fn video_detail(
        &self,
        context: &ProtocolContext,
        request: VideoDetailRequest<'_>,
    ) -> Result<VideoDetailSession, ProtocolError> {
        let auth = match request.auth {
            Some(auth) => auth,
            None => {
                Arc::new(establish_course_video_session(context, request.canvas_course_id).await?)
            }
        };
        match get_video_info(context, &auth, request.video_id).await {
            Ok(info) => Ok(VideoDetailSession { auth, info }),
            Err(ProtocolError::VideoTokenExpired) => {
                refresh_detail(context, request.canvas_course_id, request.video_id).await
            }
            Err(error) => Err(error),
        }
    }
}

async fn refresh_detail(
    context: &ProtocolContext,
    canvas_course_id: i64,
    video_id: &str,
) -> Result<VideoDetailSession, ProtocolError> {
    let auth = Arc::new(establish_course_video_session(context, canvas_course_id).await?);
    let info = get_video_info(context, &auth, video_id).await?;
    Ok(VideoDetailSession { auth, info })
}
