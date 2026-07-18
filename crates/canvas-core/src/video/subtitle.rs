use secrecy::ExposeSecret;
use serde::{Deserialize, Serialize};

use crate::{
    client::{ProtocolContext, UpstreamPurpose, read_limited_body, validate_upstream_url},
    error::ProtocolError,
    lti::CourseVideoAuth,
};

const MAX_SUBTITLE_BYTES: usize = 4 * 1024 * 1024;
const MILLIS_PER_SECOND: u64 = 1_000;
const SECONDS_PER_MINUTE: u64 = 60;
const MINUTES_PER_HOUR: u64 = 60;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubtitleDocument {
    pub srt: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SubtitleRequest {
    course_id: String,
}

#[derive(Deserialize)]
struct SubtitleResponse {
    data: Option<SubtitleData>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SubtitleData {
    #[serde(default)]
    before_assembly_list: Vec<SubtitleCue>,
}

#[derive(Deserialize)]
struct SubtitleCue {
    bg: u64,
    ed: u64,
    res: String,
}

pub async fn get_subtitle(
    context: &ProtocolContext,
    auth: &CourseVideoAuth,
    source_course_id: i64,
) -> Result<SubtitleDocument, ProtocolError> {
    let target = &context.endpoints.subtitle_detail;
    validate_upstream_url(target, UpstreamPurpose::VideoApi, &context.policy)?;
    let request = SubtitleRequest {
        course_id: source_course_id.to_string(),
    };
    let response = context
        .stateless_client
        .post(target.clone())
        .header("token", auth.token.expose_secret())
        .json(&request)
        .send()
        .await
        .map_err(|_| ProtocolError::SubtitleFailed)?;
    if matches!(response.status().as_u16(), 401 | 403) {
        return Err(ProtocolError::VideoTokenExpired);
    }
    if !response.status().is_success() {
        return Err(ProtocolError::SubtitleFailed);
    }
    let body = read_limited_body(response, MAX_SUBTITLE_BYTES).await?;
    parse_subtitle(&body)
}

fn parse_subtitle(body: &[u8]) -> Result<SubtitleDocument, ProtocolError> {
    let response: SubtitleResponse =
        serde_json::from_slice(body).map_err(|_| ProtocolError::SubtitleFailed)?;
    let cues = response
        .data
        .ok_or(ProtocolError::SubtitleMissing)?
        .before_assembly_list;
    if cues.is_empty() {
        return Err(ProtocolError::SubtitleMissing);
    }
    Ok(SubtitleDocument {
        srt: cues_to_srt(&cues),
    })
}

fn cues_to_srt(cues: &[SubtitleCue]) -> String {
    let mut output = String::new();
    for (index, cue) in cues.iter().enumerate() {
        let end = cue_end(cues, index);
        let text = cue.res.replace('\0', "");
        output.push_str(&format!(
            "{}\n{} --> {}\n{}\n\n",
            index + 1,
            format_time(cue.bg),
            format_time(end),
            text
        ));
    }
    output
}

fn cue_end(cues: &[SubtitleCue], index: usize) -> u64 {
    let cue = &cues[index];
    cues.get(index + 1)
        .map(|next| next.bg)
        .filter(|next_start| *next_start >= cue.bg)
        .unwrap_or(cue.ed.max(cue.bg))
}

fn format_time(milliseconds: u64) -> String {
    let total_seconds = milliseconds / MILLIS_PER_SECOND;
    let millis = milliseconds % MILLIS_PER_SECOND;
    let seconds = total_seconds % SECONDS_PER_MINUTE;
    let total_minutes = total_seconds / SECONDS_PER_MINUTE;
    let minutes = total_minutes % MINUTES_PER_HOUR;
    let hours = total_minutes / MINUTES_PER_HOUR;
    format!("{hours:02}:{minutes:02}:{seconds:02},{millis:03}")
}

#[cfg(test)]
mod tests {
    use super::{SubtitleCue, cues_to_srt, format_time};

    #[test]
    fn formats_srt_timestamp() {
        assert_eq!(format_time(3_723_045), "01:02:03,045");
    }

    #[test]
    fn converts_original_cues_and_uses_next_start_as_end() {
        let cues = vec![
            SubtitleCue {
                bg: 1_000,
                ed: 1_500,
                res: "第一句".to_owned(),
            },
            SubtitleCue {
                bg: 2_000,
                ed: 3_000,
                res: "第二句".to_owned(),
            },
        ];
        assert_eq!(
            cues_to_srt(&cues),
            "1\n00:00:01,000 --> 00:00:02,000\n第一句\n\n2\n00:00:02,000 --> 00:00:03,000\n第二句\n\n"
        );
    }
}
