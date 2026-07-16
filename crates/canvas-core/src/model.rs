use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserIdentity {
    pub stable_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CanvasCourse {
    pub id: i64,
    pub name: String,
    pub course_code: String,
    pub term_name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CanvasVideo {
    pub id: String,
    pub name: String,
    pub starts_at: Option<String>,
    pub ends_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VideoTrack {
    pub id: String,
    pub label: String,
}

#[cfg(test)]
mod tests {
    use super::VideoTrack;

    #[test]
    fn public_track_model_contains_no_upstream_url() {
        let value = serde_json::to_value(VideoTrack {
            id: "track-1".to_owned(),
            label: "轨道 1".to_owned(),
        })
        .expect("video track should serialize");

        assert!(value.get("url").is_none());
    }
}
