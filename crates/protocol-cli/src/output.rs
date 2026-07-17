use canvas_core::{
    canvas::{CanvasCourse, CanvasSessionStatus, UserIdentity},
    redaction::Redactor,
    video::{CanvasVideo, VideoTrack},
};
use qrcode::{QrCode, render::unicode};
use secrecy::{ExposeSecret, SecretString};
use uuid::Uuid;

use crate::{error::CliError, report::ValidationReport};

pub struct Output {
    json_output: bool,
    redactor: Redactor,
}

impl Output {
    pub fn new(json_output: bool) -> Self {
        Self {
            json_output,
            redactor: Redactor::new(random_redaction_key()),
        }
    }

    pub fn stage(&self, message: &str) {
        eprintln!("[phase-1] {message}");
    }

    pub fn qr_code(&self, url: &SecretString) -> Result<(), CliError> {
        let code = QrCode::new(url.expose_secret().as_bytes()).map_err(|_| CliError::QrRender)?;
        let rendered = code.render::<unicode::Dense1x2>().quiet_zone(true).build();
        eprintln!("请使用 jAccount 扫描以下二维码：\n{rendered}");
        Ok(())
    }

    pub fn unknown_event(&self, event_type: &str) {
        let event_hash = self.redactor.hash_identifier(event_type);
        eprintln!("忽略未知 jAccount 事件：event_hash={event_hash}");
    }

    pub fn canvas_status(&self, status: &CanvasSessionStatus) {
        eprintln!(
            "Canvas Session: authenticated={} host={} cookies={:?}",
            status.authenticated,
            status.final_host.as_deref().unwrap_or("<missing>"),
            status.cookie_names
        );
    }

    pub fn identity(&self, identity: &UserIdentity) {
        eprintln!("{}", format_identity_summary(identity, &self.redactor));
    }

    pub fn courses(&self, courses: &[CanvasCourse]) {
        eprintln!("{}", format_course_summary(courses));
    }

    pub fn videos(&self, videos: &[CanvasVideo]) {
        eprintln!("{}", format_video_summary(videos, &self.redactor));
    }

    pub fn tracks(&self, tracks: &[VideoTrack]) -> Result<(), CliError> {
        eprintln!("{}", format_track_summary(tracks, &self.redactor)?);
        Ok(())
    }

    pub fn report(&self, report: &ValidationReport) -> Result<(), CliError> {
        if self.json_output {
            let json = serde_json::to_string_pretty(report).map_err(|_| CliError::Output)?;
            println!("{json}");
            return Ok(());
        }
        eprintln!("Go/No-Go: {:?}", report.decision);
        for (step, status) in &report.steps {
            eprintln!("  {step:?}: {status:?}");
        }
        Ok(())
    }
}

fn random_redaction_key() -> [u8; 32] {
    let first = Uuid::new_v4();
    let second = Uuid::new_v4();
    let mut key = [0_u8; 32];
    key[..16].copy_from_slice(first.as_bytes());
    key[16..].copy_from_slice(second.as_bytes());
    key
}

fn format_course_summary(courses: &[CanvasCourse]) -> String {
    format!("课程发现成功：{} 门", courses.len())
}

fn format_identity_summary(identity: &UserIdentity, redactor: &Redactor) -> String {
    let value = identity.stable_id.expose_secret();
    let correlation_hash = redactor.hash_identifier(value);
    let whitelist_hash =
        canvas_core::identity::hash_stable_id(value).unwrap_or_else(|_| "unavailable".to_owned());
    format!(
        "稳定身份：source={:?} stable_id_hash={correlation_hash} whitelist_hash={whitelist_hash}",
        identity.source
    )
}

fn format_video_summary(videos: &[CanvasVideo], redactor: &Redactor) -> String {
    let mut lines = vec![format!("录像列表：{} 个", videos.len())];
    lines.extend(videos.iter().map(|video| {
        format!(
            "  video_id_hash={} started_at={}",
            redactor.hash_identifier(&video.id),
            video.started_at.as_deref().unwrap_or("-")
        )
    }));
    lines.join("\n")
}

fn format_track_summary(tracks: &[VideoTrack], redactor: &Redactor) -> Result<String, CliError> {
    let mut lines = vec![format!("视频轨道：{} 条", tracks.len())];
    for track in tracks {
        let metadata = track
            .sanitized_upstream(redactor)
            .map_err(CliError::Protocol)?;
        lines.push(format!(
            "  track_id_hash={} kind={:?} {}",
            redactor.hash_identifier(&track.id),
            track.kind,
            metadata
        ));
    }
    Ok(lines.join("\n"))
}

#[cfg(test)]
mod tests {
    use canvas_core::{
        canvas::{CanvasCourse, CanvasTerm},
        redaction::Redactor,
        video::{CanvasVideo, VideoTrack, VideoTrackInput, VideoTrackKind},
    };
    use secrecy::SecretString;

    use super::{
        format_course_summary, format_identity_summary, format_track_summary, format_video_summary,
    };

    #[test]
    fn identity_summary_exposes_only_hashes() {
        let identity = canvas_core::canvas::UserIdentity {
            stable_id: SecretString::from("private-stable-id".to_owned()),
            account: None,
            display_name: None,
            source: canvas_core::canvas::IdentitySource::MySjtuAccount,
        };
        let summary = format_identity_summary(&identity, &Redactor::new([7_u8; 32]));

        assert!(summary.contains("whitelist_hash=sha256:"));
        assert!(!summary.contains("private-stable-id"));
    }

    #[test]
    fn course_summary_exposes_only_the_count() {
        let courses = vec![CanvasCourse {
            id: 12345,
            name: "private-course-name".to_owned(),
            course_code: "private-course-code".to_owned(),
            term: Some(CanvasTerm {
                name: "private-term".to_owned(),
            }),
        }];

        let summary = format_course_summary(&courses);

        assert_eq!(summary, "课程发现成功：1 门");
        assert!(!summary.contains("12345"));
        assert!(!summary.contains("private-course"));
        assert!(!summary.contains("private-term"));
    }

    #[test]
    fn video_and_track_summaries_exclude_private_names() {
        let redactor = Redactor::new([7_u8; 32]);
        let videos = vec![CanvasVideo {
            id: "private-video-id".to_owned(),
            name: "private-video-name".to_owned(),
            started_at: Some("2026-07-17T08:00:00Z".to_owned()),
            ended_at: None,
        }];
        let tracks = vec![VideoTrack::new(VideoTrackInput {
            id: "private-track-id".to_owned(),
            kind: VideoTrackKind::Screen,
            suggested_filename: "private-filename.mp4".to_owned(),
            upstream_url: SecretString::from("https://live.sjtu.edu.cn/private-path?secret=1"),
        })];

        let video_summary = format_video_summary(&videos, &redactor);
        let track_summary = format_track_summary(&tracks, &redactor).expect("fixture URL is valid");

        assert!(!video_summary.contains("private-video-name"));
        assert!(!video_summary.contains("private-video-id"));
        assert!(!track_summary.contains("private-filename"));
        assert!(!track_summary.contains("private-path"));
        assert!(!track_summary.contains("secret=1"));
    }
}
