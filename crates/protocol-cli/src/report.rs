use std::{collections::BTreeMap, path::Path};

use serde::Serialize;

pub const REFERENCE_COMMIT: &str = "b5d895af57aaa74dfd53cef80dfb64c76c023c20";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StepName {
    #[serde(rename = "jaccount_uuid")]
    JAccountUuid,
    #[serde(rename = "jaccount_websocket")]
    JAccountWebsocket,
    QrCode,
    ExpressLogin,
    CanvasLogin,
    Identity,
    CourseDiscovery,
    LtiLaunch,
    VideoList,
    VideoDetail,
    RangeProbe,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StepStatus {
    Passed,
    Failed,
    NotRun,
    Blocked,
    RequiresPersonalAccessToken,
    CookieSessionRejected,
    CsrfRequired,
    UnsupportedResponse,
    UpstreamChanged,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum GoNoGoDecision {
    GoA,
    GoB,
    NoGoC,
    Undetermined,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct SanitizedMetadata {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub video_host: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supports_range: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ValidationReport {
    pub started_at: String,
    pub reference_commit: &'static str,
    pub decision: GoNoGoDecision,
    pub steps: BTreeMap<StepName, StepStatus>,
    pub sanitized_metadata: SanitizedMetadata,
}

impl ValidationReport {
    pub fn new(started_at: String) -> Self {
        let steps = all_steps()
            .into_iter()
            .map(|step| (step, StepStatus::NotRun))
            .collect();
        Self {
            started_at,
            reference_commit: REFERENCE_COMMIT,
            decision: GoNoGoDecision::Undetermined,
            steps,
            sanitized_metadata: SanitizedMetadata::default(),
        }
    }

    pub fn set_step(&mut self, step: StepName, status: StepStatus) {
        self.steps.insert(step, status);
    }

    pub fn set_video_metadata(&mut self, host: String, supports_range: bool) {
        self.sanitized_metadata.video_host = Some(host);
        self.sanitized_metadata.supports_range = Some(supports_range);
    }

    pub fn block_steps(&mut self, steps: &[StepName]) {
        for step in steps {
            if self.steps.get(step) == Some(&StepStatus::NotRun) {
                self.steps.insert(*step, StepStatus::Blocked);
            }
        }
    }

    pub fn finalize_decision(&mut self) {
        let critical = critical_steps()
            .into_iter()
            .filter_map(|step| self.steps.get(&step))
            .copied()
            .collect::<Vec<_>>();
        if critical.contains(&StepStatus::Failed) {
            self.decision = GoNoGoDecision::NoGoC;
            return;
        }
        if !critical.iter().all(|status| *status == StepStatus::Passed) {
            self.decision = GoNoGoDecision::Undetermined;
            return;
        }
        self.decision = match self.steps.get(&StepName::CourseDiscovery) {
            Some(StepStatus::Passed) => GoNoGoDecision::GoA,
            Some(
                StepStatus::RequiresPersonalAccessToken
                | StepStatus::CookieSessionRejected
                | StepStatus::CsrfRequired
                | StepStatus::UnsupportedResponse,
            ) => GoNoGoDecision::GoB,
            _ => GoNoGoDecision::Undetermined,
        };
    }

    pub async fn write_json(&self, path: &Path) -> Result<(), ReportWriteError> {
        let parent = path.parent().ok_or(ReportWriteError::InvalidPath)?;
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|_| ReportWriteError::CreateDirectory)?;
        let bytes = serde_json::to_vec_pretty(self).map_err(|_| ReportWriteError::Serialize)?;
        tokio::fs::write(path, bytes)
            .await
            .map_err(|_| ReportWriteError::Write)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ReportWriteError {
    #[error("protocol report path is invalid")]
    InvalidPath,
    #[error("protocol report directory could not be created")]
    CreateDirectory,
    #[error("protocol report serialization failed")]
    Serialize,
    #[error("protocol report could not be written")]
    Write,
}

fn all_steps() -> [StepName; 11] {
    [
        StepName::JAccountUuid,
        StepName::JAccountWebsocket,
        StepName::QrCode,
        StepName::ExpressLogin,
        StepName::CanvasLogin,
        StepName::Identity,
        StepName::CourseDiscovery,
        StepName::LtiLaunch,
        StepName::VideoList,
        StepName::VideoDetail,
        StepName::RangeProbe,
    ]
}

fn critical_steps() -> [StepName; 9] {
    [
        StepName::JAccountUuid,
        StepName::JAccountWebsocket,
        StepName::QrCode,
        StepName::ExpressLogin,
        StepName::CanvasLogin,
        StepName::LtiLaunch,
        StepName::VideoList,
        StepName::VideoDetail,
        StepName::RangeProbe,
    ]
}
