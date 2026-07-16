use std::time::Duration;

mod login;

use canvas_core::{
    ProtocolError,
    canvas::{CourseDiscoveryOutcome, discover_courses, establish_canvas_session, probe_identity},
    client::{ProtocolConfig, ProtocolContext},
    video::{
        VideoCatalogSession, get_video_info, list_course_videos_with_refresh, probe_video_track,
    },
};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

use crate::{
    args::{Cli, Command, CourseArgs},
    error::CliError,
    output::Output,
    report::{StepName, StepStatus, ValidationReport},
};

const API_TIMEOUT_SECONDS: u64 = 30;

pub struct Execution {
    pub report: ValidationReport,
    pub error: Option<CliError>,
}

struct Workflow<'a> {
    context: ProtocolContext,
    output: &'a Output,
    report: ValidationReport,
    login_timeout: Duration,
    no_course_discovery: bool,
}

pub async fn run(cli: &Cli, output: &Output) -> Result<Execution, CliError> {
    let started_at = OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .map_err(|_| CliError::Timestamp)?;
    let api_timeout = Duration::from_secs(cli.timeout_seconds.min(API_TIMEOUT_SECONDS));
    let context = ProtocolContext::new(ProtocolConfig::production(api_timeout))?;
    let mut workflow = Workflow {
        context,
        output,
        report: ValidationReport::new(started_at),
        login_timeout: Duration::from_secs(cli.timeout_seconds),
        no_course_discovery: cli.no_course_discovery,
    };
    let result = workflow.execute(&cli.command).await;
    workflow.report.finalize_decision();
    Ok(Execution {
        report: workflow.report,
        error: result.err(),
    })
}

impl Workflow<'_> {
    async fn execute(&mut self, command: &Command) -> Result<(), CliError> {
        self.authenticate_canvas().await?;
        self.probe_identity().await;
        match command {
            Command::Login => Ok(()),
            Command::DiscoverCourses => {
                self.discover_courses().await;
                Ok(())
            }
            Command::InspectCourse(args) | Command::Full(args) => {
                if !self.no_course_discovery {
                    self.discover_courses().await;
                }
                self.inspect_course(args).await
            }
        }
    }

    async fn authenticate_canvas(&mut self) -> Result<(), CliError> {
        self.output.stage("启动 jAccount 后端二维码登录");
        let session = match self.login_jaccount().await {
            Ok(session) => session,
            Err(error) => {
                self.block_after_login();
                return Err(error);
            }
        };
        self.output.stage("建立 Canvas Web Session");
        match establish_canvas_session(&self.context, &session.ja_auth_cookie).await {
            Ok(status) => {
                self.report
                    .set_step(StepName::CanvasLogin, StepStatus::Passed);
                self.output.canvas_status(&status);
                Ok(())
            }
            Err(error) => {
                self.report
                    .set_step(StepName::CanvasLogin, StepStatus::Failed);
                self.block_after_canvas();
                Err(error.into())
            }
        }
    }

    async fn probe_identity(&mut self) {
        self.output.stage("探测稳定用户身份");
        match probe_identity(&self.context).await {
            Ok(identity) => {
                self.report.set_step(StepName::Identity, StepStatus::Passed);
                self.output.identity(&identity);
            }
            Err(_) => self.report.set_step(StepName::Identity, StepStatus::Failed),
        }
    }

    async fn discover_courses(&mut self) {
        self.output.stage("实验 Cookie Session 课程发现");
        match discover_courses(&self.context).await {
            Ok(outcome) => self.record_course_outcome(outcome),
            Err(_) => self
                .report
                .set_step(StepName::CourseDiscovery, StepStatus::Failed),
        }
    }

    fn record_course_outcome(&mut self, outcome: CourseDiscoveryOutcome) {
        let status = match outcome {
            CourseDiscoveryOutcome::Success { courses, .. } => {
                self.output.courses(&courses);
                StepStatus::Passed
            }
            CourseDiscoveryOutcome::RequiresPersonalAccessToken => {
                StepStatus::RequiresPersonalAccessToken
            }
            CourseDiscoveryOutcome::CookieSessionRejected => StepStatus::CookieSessionRejected,
            CourseDiscoveryOutcome::CsrfRequired => StepStatus::CsrfRequired,
            CourseDiscoveryOutcome::UnsupportedResponse => StepStatus::UnsupportedResponse,
            CourseDiscoveryOutcome::UpstreamChanged => StepStatus::UpstreamChanged,
        };
        self.report.set_step(StepName::CourseDiscovery, status);
    }

    async fn inspect_course(&mut self, args: &CourseArgs) -> Result<(), CliError> {
        self.output.stage("执行 LTI 并获取录像列表");
        let catalog = match list_course_videos_with_refresh(&self.context, args.course_id).await {
            Ok(catalog) => {
                self.report
                    .set_step(StepName::LtiLaunch, StepStatus::Passed);
                self.report
                    .set_step(StepName::VideoList, StepStatus::Passed);
                catalog
            }
            Err(error) => {
                self.mark_catalog_error(&error);
                return Err(error.into());
            }
        };
        self.output.videos(&catalog.videos);
        self.inspect_video(args, catalog).await
    }

    async fn inspect_video(
        &mut self,
        args: &CourseArgs,
        catalog: VideoCatalogSession,
    ) -> Result<(), CliError> {
        let video = select_video(&catalog, args.video_id.as_deref())?;
        let info = get_video_info(&self.context, &catalog.auth, &video.id)
            .await
            .inspect_err(|_| {
                self.report
                    .set_step(StepName::VideoDetail, StepStatus::Failed);
                self.report.block_steps(&[StepName::RangeProbe]);
            })?;
        self.report
            .set_step(StepName::VideoDetail, StepStatus::Passed);
        self.output.tracks(&info.tracks)?;
        let track = info.tracks.first().ok_or(CliError::NoVideoAvailable)?;
        match probe_video_track(&self.context, track).await {
            Ok(probe) => {
                self.report
                    .set_step(StepName::RangeProbe, StepStatus::Passed);
                self.report
                    .set_video_metadata(probe.host, probe.supports_range);
                Ok(())
            }
            Err(error) => {
                self.report
                    .set_step(StepName::RangeProbe, StepStatus::Failed);
                Err(error.into())
            }
        }
    }

    fn mark_catalog_error(&mut self, error: &ProtocolError) {
        if matches!(
            error,
            ProtocolError::VideoListFailed | ProtocolError::VideoTokenExpired
        ) {
            self.report
                .set_step(StepName::LtiLaunch, StepStatus::Passed);
            self.report
                .set_step(StepName::VideoList, StepStatus::Failed);
            self.report
                .block_steps(&[StepName::VideoDetail, StepName::RangeProbe]);
            return;
        }
        self.report
            .set_step(StepName::LtiLaunch, StepStatus::Failed);
        self.report.block_steps(&[
            StepName::VideoList,
            StepName::VideoDetail,
            StepName::RangeProbe,
        ]);
    }

    fn block_after_login(&mut self) {
        self.report.block_steps(&[
            StepName::CanvasLogin,
            StepName::Identity,
            StepName::CourseDiscovery,
            StepName::LtiLaunch,
            StepName::VideoList,
            StepName::VideoDetail,
            StepName::RangeProbe,
        ]);
    }

    fn block_after_canvas(&mut self) {
        self.report.block_steps(&[
            StepName::Identity,
            StepName::CourseDiscovery,
            StepName::LtiLaunch,
            StepName::VideoList,
            StepName::VideoDetail,
            StepName::RangeProbe,
        ]);
    }
}

fn select_video<'a>(
    catalog: &'a VideoCatalogSession,
    requested_id: Option<&str>,
) -> Result<&'a canvas_core::video::CanvasVideo, CliError> {
    match requested_id {
        Some(requested_id) => catalog
            .videos
            .iter()
            .find(|video| video.id == requested_id)
            .ok_or(CliError::RequestedVideoUnavailable),
        None => catalog.videos.first().ok_or(CliError::NoVideoAvailable),
    }
}
