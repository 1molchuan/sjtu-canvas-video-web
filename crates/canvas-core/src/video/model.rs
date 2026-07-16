use secrecy::{ExposeSecret, SecretString};
use url::Url;

use crate::{
    error::ProtocolError,
    redaction::{Redactor, SanitizedUrl},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanvasVideo {
    pub id: String,
    pub name: String,
    pub started_at: Option<String>,
    pub ended_at: Option<String>,
}

#[derive(Debug)]
pub struct VideoInfo {
    pub id: String,
    pub name: String,
    pub tracks: Vec<VideoTrack>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VideoTrackKind {
    Camera,
    Screen,
    Mixed,
    Unknown,
}

#[derive(Debug)]
pub struct VideoTrack {
    pub id: String,
    pub kind: VideoTrackKind,
    pub suggested_filename: String,
    pub(crate) upstream_url: SecretString,
}

pub struct VideoTrackInput {
    pub id: String,
    pub kind: VideoTrackKind,
    pub suggested_filename: String,
    pub upstream_url: SecretString,
}

impl VideoTrack {
    pub fn new(input: VideoTrackInput) -> Self {
        Self {
            id: input.id,
            kind: input.kind,
            suggested_filename: input.suggested_filename,
            upstream_url: input.upstream_url,
        }
    }

    pub fn sanitized_upstream(&self, redactor: &Redactor) -> Result<SanitizedUrl, ProtocolError> {
        let url = Url::parse(self.upstream_url.expose_secret())
            .map_err(|_| ProtocolError::RangeProbeFailed)?;
        Ok(redactor.sanitize_url(&url))
    }
}
