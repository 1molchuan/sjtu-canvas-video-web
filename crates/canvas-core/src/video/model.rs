use secrecy::SecretString;

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
}
