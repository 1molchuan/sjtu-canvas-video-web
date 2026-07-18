use secrecy::{ExposeSecret, SecretString};
use serde::Deserialize;
use serde_json::Value;
use url::Url;

use crate::{
    client::{ProtocolContext, UpstreamPurpose, read_limited_body, validate_upstream_url},
    error::ProtocolError,
    lti::CourseVideoAuth,
};

use super::{VideoInfo, VideoTrack, VideoTrackInput, VideoTrackKind};

const MAX_VIDEO_DETAIL_BYTES: usize = 4 * 1024 * 1024;
const MAX_FILENAME_CHARS: usize = 120;

#[derive(Deserialize)]
struct DetailResponse {
    data: Option<RawDetail>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawDetail {
    cour_id: i64,
    #[serde(default)]
    vide_name: String,
    video_play_response_vo_list: Vec<RawTrack>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawTrack {
    id: Value,
    #[serde(default)]
    track_type: Option<String>,
    rtmp_url_hdv: String,
}

struct TrackBuilder<'a> {
    context: &'a ProtocolContext,
    video_name: &'a str,
}

pub async fn get_video_info(
    context: &ProtocolContext,
    auth: &CourseVideoAuth,
    video_id: &str,
) -> Result<VideoInfo, ProtocolError> {
    let target = &context.endpoints.video_detail;
    validate_upstream_url(target, UpstreamPurpose::VideoApi, &context.policy)?;
    let response = context
        .stateless_client
        .post(target.clone())
        .header("token", auth.token.expose_secret())
        .form(&[
            ("playTypeHls", "true"),
            ("id", video_id),
            ("isAudit", "true"),
        ])
        .send()
        .await
        .map_err(|_| ProtocolError::VideoDetailFailed)?;
    if matches!(response.status().as_u16(), 401 | 403) {
        return Err(ProtocolError::VideoTokenExpired);
    }
    if !response.status().is_success() {
        return Err(ProtocolError::VideoDetailFailed);
    }
    let body = read_limited_body(response, MAX_VIDEO_DETAIL_BYTES).await?;
    parse_detail(context, video_id, &body)
}

fn parse_detail(
    context: &ProtocolContext,
    video_id: &str,
    body: &[u8],
) -> Result<VideoInfo, ProtocolError> {
    let response: DetailResponse =
        serde_json::from_slice(body).map_err(|_| ProtocolError::VideoDetailFailed)?;
    let detail = response.data.ok_or(ProtocolError::VideoDetailFailed)?;
    let builder = TrackBuilder {
        context,
        video_name: &detail.vide_name,
    };
    let tracks = detail
        .video_play_response_vo_list
        .into_iter()
        .enumerate()
        .map(|(index, track)| builder.build(index, track))
        .collect::<Result<Vec<_>, _>>()?;
    if tracks.is_empty() {
        return Err(ProtocolError::VideoTrackMissing);
    }
    Ok(VideoInfo {
        id: video_id.to_owned(),
        name: detail.vide_name,
        source_course_id: detail.cour_id,
        tracks,
    })
}

impl TrackBuilder<'_> {
    fn build(&self, index: usize, track: RawTrack) -> Result<VideoTrack, ProtocolError> {
        let url =
            Url::parse(&track.rtmp_url_hdv).map_err(|_| ProtocolError::InvalidUpstreamUrl {
                purpose: UpstreamPurpose::VideoContent,
                reason: "video track URL is invalid",
            })?;
        validate_upstream_url(&url, UpstreamPurpose::VideoContent, &self.context.policy)?;
        let id = scalar_id(&track.id).ok_or(ProtocolError::VideoTrackMissing)?;
        let kind = track_kind(track.track_type.as_deref());
        let suggested_filename = suggested_filename(self.video_name, index, kind);
        Ok(VideoTrack::new(VideoTrackInput {
            id,
            kind,
            suggested_filename,
            upstream_url: SecretString::from(url.to_string()),
        }))
    }
}

fn scalar_id(value: &Value) -> Option<String> {
    match value {
        Value::String(value) if !value.is_empty() => Some(value.clone()),
        Value::Number(value) => Some(value.to_string()),
        _ => None,
    }
}

fn track_kind(value: Option<&str>) -> VideoTrackKind {
    match value.map(str::to_ascii_lowercase).as_deref() {
        Some("camera") => VideoTrackKind::Camera,
        Some("screen") => VideoTrackKind::Screen,
        Some("mixed") => VideoTrackKind::Mixed,
        _ => VideoTrackKind::Unknown,
    }
}

fn suggested_filename(name: &str, index: usize, kind: VideoTrackKind) -> String {
    let base = sanitize_filename_component(name);
    let label = match kind {
        VideoTrackKind::Camera => "camera",
        VideoTrackKind::Screen => "screen",
        VideoTrackKind::Mixed => "mixed",
        VideoTrackKind::Unknown => "track",
    };
    format!("{base}-{label}-{}.mp4", index + 1)
}

pub fn sanitize_filename_component(value: &str) -> String {
    let cleaned = value
        .chars()
        .take(MAX_FILENAME_CHARS)
        .map(|character| match character {
            character if character.is_alphanumeric() => character,
            '-' | '_' | ' ' => character,
            _ => '_',
        })
        .collect::<String>();
    let trimmed = cleaned.trim_matches([' ', '.', '_']);
    if trimmed.is_empty() {
        return "video".to_owned();
    }
    trimmed.to_owned()
}
