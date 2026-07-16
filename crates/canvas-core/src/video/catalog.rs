use reqwest::header::REFERER;
use secrecy::ExposeSecret;
use serde::{Deserialize, Serialize};

use crate::{
    client::{ProtocolContext, UpstreamPurpose, read_limited_body, validate_upstream_url},
    error::ProtocolError,
    lti::{CourseVideoAuth, establish_course_video_session},
};

use super::CanvasVideo;

const MAX_VIDEO_LIST_BYTES: usize = 4 * 1024 * 1024;

#[derive(Debug)]
pub struct VideoCatalogSession {
    pub auth: CourseVideoAuth,
    pub videos: Vec<CanvasVideo>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct VideoListRequest<'a> {
    canvas_course_id: &'a str,
}

#[derive(Deserialize)]
struct VideoListResponse {
    data: Option<VideoListData>,
}

#[derive(Deserialize)]
struct VideoListData {
    records: Vec<RawVideo>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawVideo {
    video_id: String,
    video_name: String,
    #[serde(default)]
    course_begin_time: Option<String>,
    #[serde(default)]
    course_end_time: Option<String>,
}

pub async fn list_course_videos(
    context: &ProtocolContext,
    auth: &CourseVideoAuth,
) -> Result<Vec<CanvasVideo>, ProtocolError> {
    let target = &context.endpoints.video_list;
    validate_upstream_url(target, UpstreamPurpose::VideoApi, &context.policy)?;
    let request = VideoListRequest {
        canvas_course_id: &auth.video_course_id,
    };
    let response = context
        .stateless_client
        .post(target.clone())
        .header(REFERER, context.endpoints.video_ui_referer.as_str())
        .header("token", auth.token.expose_secret())
        .json(&request)
        .send()
        .await
        .map_err(|_| ProtocolError::VideoListFailed)?;
    if matches!(response.status().as_u16(), 401 | 403) {
        return Err(ProtocolError::VideoTokenExpired);
    }
    if !response.status().is_success() {
        return Err(ProtocolError::VideoListFailed);
    }
    let body = read_limited_body(response, MAX_VIDEO_LIST_BYTES).await?;
    parse_video_list(&body)
}

pub async fn list_course_videos_with_refresh(
    context: &ProtocolContext,
    canvas_course_id: i64,
) -> Result<VideoCatalogSession, ProtocolError> {
    let first_auth = establish_course_video_session(context, canvas_course_id).await?;
    match list_course_videos(context, &first_auth).await {
        Ok(videos) => Ok(VideoCatalogSession {
            auth: first_auth,
            videos,
        }),
        Err(ProtocolError::VideoTokenExpired) => {
            let auth = establish_course_video_session(context, canvas_course_id).await?;
            let videos = list_course_videos(context, &auth).await?;
            Ok(VideoCatalogSession { auth, videos })
        }
        Err(error) => Err(error),
    }
}

fn parse_video_list(body: &[u8]) -> Result<Vec<CanvasVideo>, ProtocolError> {
    let response: VideoListResponse =
        serde_json::from_slice(body).map_err(|_| ProtocolError::VideoListFailed)?;
    let records = response.data.ok_or(ProtocolError::VideoListFailed)?.records;
    Ok(records
        .into_iter()
        .map(|video| CanvasVideo {
            id: video.video_id,
            name: video.video_name,
            started_at: video.course_begin_time,
            ended_at: video.course_end_time,
        })
        .collect())
}
