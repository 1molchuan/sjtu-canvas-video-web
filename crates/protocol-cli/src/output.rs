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
        let stable_hash = self
            .redactor
            .hash_identifier(identity.stable_id.expose_secret());
        eprintln!(
            "稳定身份：source={:?} stable_id_hash={stable_hash}",
            identity.source
        );
    }

    pub fn courses(&self, courses: &[CanvasCourse]) {
        eprintln!("课程发现成功：{} 门", courses.len());
        for course in courses {
            let term = course
                .term
                .as_ref()
                .map(|term| term.name.as_str())
                .unwrap_or("-");
            eprintln!(
                "  course_id={} code={} term={} name={}",
                course.id, course.course_code, term, course.name
            );
        }
    }

    pub fn videos(&self, videos: &[CanvasVideo]) {
        eprintln!("录像列表：{} 个", videos.len());
        for video in videos {
            let id_hash = self.redactor.hash_identifier(&video.id);
            eprintln!(
                "  video_id_hash={} started_at={} name={}",
                id_hash,
                video.started_at.as_deref().unwrap_or("-"),
                video.name
            );
        }
    }

    pub fn tracks(&self, tracks: &[VideoTrack]) -> Result<(), CliError> {
        eprintln!("视频轨道：{} 条", tracks.len());
        for track in tracks {
            let metadata = track
                .sanitized_upstream(&self.redactor)
                .map_err(CliError::Protocol)?;
            eprintln!(
                "  track_id_hash={} kind={:?} filename={} {}",
                self.redactor.hash_identifier(&track.id),
                track.kind,
                track.suggested_filename,
                metadata
            );
        }
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
