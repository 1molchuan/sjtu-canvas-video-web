use std::time::Duration;

use canvas_core::{
    ProtocolError,
    client::ProtocolContext,
    jaccount::{JAccountSession, QrLoginOptions, QrLoginProgress, login_with_qr},
};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::{
    error::CliError,
    output::Output,
    report::{StepName, StepStatus},
};

use super::Workflow;

const QR_REFRESH_SECONDS: u64 = 25;

impl Workflow<'_> {
    pub(super) async fn login_jaccount(&mut self) -> Result<JAccountSession, CliError> {
        let result = run_login_loop(&self.context, self.output, self.login_timeout).await?;
        let (result, qr_seen) = result;
        if qr_seen {
            self.report.set_step(StepName::QrCode, StepStatus::Passed);
        }
        self.finish_login(result, qr_seen)
    }

    fn finish_login(
        &mut self,
        result: Result<JAccountSession, ProtocolError>,
        qr_seen: bool,
    ) -> Result<JAccountSession, CliError> {
        match result {
            Ok(session) => {
                self.mark_login_success(qr_seen);
                Ok(session)
            }
            Err(error) => {
                self.mark_login_error(&error, qr_seen);
                Err(error.into())
            }
        }
    }

    fn mark_login_success(&mut self, qr_seen: bool) {
        self.report
            .set_step(StepName::JAccountUuid, StepStatus::Passed);
        self.report
            .set_step(StepName::JAccountWebsocket, StepStatus::Passed);
        self.report
            .set_step(StepName::ExpressLogin, StepStatus::Passed);
        if !qr_seen {
            self.report.set_step(StepName::QrCode, StepStatus::Failed);
        }
    }

    fn mark_login_error(&mut self, error: &ProtocolError, qr_seen: bool) {
        match error {
            ProtocolError::JAccountUuidUnavailable | ProtocolError::JAccountUuidRequestFailed => {
                self.report
                    .set_step(StepName::JAccountUuid, StepStatus::Failed);
            }
            ProtocolError::JAccountExpressLoginFailed | ProtocolError::JAccountCookieMissing => {
                self.mark_pre_express_success();
                self.report
                    .set_step(StepName::ExpressLogin, StepStatus::Failed);
            }
            _ => {
                self.report
                    .set_step(StepName::JAccountUuid, StepStatus::Passed);
                self.report
                    .set_step(StepName::JAccountWebsocket, StepStatus::Failed);
            }
        }
        if qr_seen {
            self.report.set_step(StepName::QrCode, StepStatus::Passed);
        }
    }

    fn mark_pre_express_success(&mut self) {
        self.report
            .set_step(StepName::JAccountUuid, StepStatus::Passed);
        self.report
            .set_step(StepName::JAccountWebsocket, StepStatus::Passed);
    }
}

async fn run_login_loop(
    context: &ProtocolContext,
    output: &Output,
    login_timeout: Duration,
) -> Result<(Result<JAccountSession, ProtocolError>, bool), CliError> {
    let (sender, mut receiver) = mpsc::unbounded_channel();
    let cancellation = CancellationToken::new();
    let options = QrLoginOptions {
        timeout: login_timeout,
        refresh_interval: Duration::from_secs(QR_REFRESH_SECONDS),
        cancellation: cancellation.clone(),
    };
    let login = login_with_qr(context, options, sender);
    let interrupt = tokio::signal::ctrl_c();
    tokio::pin!(login, interrupt);
    let mut qr_seen = false;
    loop {
        tokio::select! {
            result = &mut login => return Ok((result, qr_seen)),
            event = receiver.recv() => {
                if let Err(error) = handle_event(output, event, &mut qr_seen) {
                    cancellation.cancel();
                    let _ = login.await;
                    return Err(error);
                }
            },
            signal = &mut interrupt => {
                cancellation.cancel();
                let result = login.await;
                if signal.is_err() {
                    return Err(cancelled_error());
                }
                return Ok((result, qr_seen));
            }
        }
    }
}

fn handle_event(
    output: &Output,
    event: Option<QrLoginProgress>,
    qr_seen: &mut bool,
) -> Result<(), CliError> {
    match event {
        Some(QrLoginProgress::QrReady { url }) => {
            output.qr_code(&url)?;
            *qr_seen = true;
        }
        Some(QrLoginProgress::Expired) => output.stage("jAccount 二维码已过期"),
        Some(QrLoginProgress::UnknownEvent { event_type }) => output.unknown_event(&event_type),
        None => {}
    }
    Ok(())
}

fn cancelled_error() -> CliError {
    CliError::Protocol(ProtocolError::JAccountLoginCancelled)
}
