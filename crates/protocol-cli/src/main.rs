use std::{path::Path, process::ExitCode};

use clap::Parser;
use protocol_cli::{
    args::{Cli, Command},
    error::CliError,
    gate::ensure_real_protocol_enabled,
    output::Output,
    workflow,
};
use tracing_subscriber::EnvFilter;

const REPORT_PATH: &str = ".local/protocol-report.json";

#[tokio::main]
async fn main() -> ExitCode {
    let cli = Cli::parse();
    init_tracing(cli.debug);
    if ensure_real_protocol_enabled().is_err() {
        eprintln!("真实协议验证默认关闭。请先设置 SJTU_REAL_PROTOCOL_TEST=1。");
        return ExitCode::from(2);
    }
    let output = Output::new(cli.json_output);
    match execute(&cli, &output).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            tracing::error!(error_class = error.class(), "protocol validation failed");
            eprintln!("验证失败：{}", error);
            ExitCode::FAILURE
        }
    }
}

async fn execute(cli: &Cli, output: &Output) -> Result<(), CliError> {
    let execution = workflow::run(cli, output).await?;
    let report_error = if matches!(cli.command, Command::Full(_)) {
        let result = execution.report.write_json(Path::new(REPORT_PATH)).await;
        if result.is_ok() {
            eprintln!("脱敏报告已写入 {REPORT_PATH}");
        }
        result.err().map(CliError::Report)
    } else {
        None
    };
    let output_result = output.report(&execution.report);
    if let Some(error) = execution.error {
        if let Some(report_error) = report_error {
            eprintln!("报告写入同时失败：{}", report_error);
        }
        output_result?;
        return Err(error);
    }
    output_result?;
    report_error.map_or(Ok(()), Err)
}

fn init_tracing(debug: bool) {
    let level = if debug { "debug" } else { "info" };
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::new(level))
        .with_target(false)
        .without_time()
        .init();
}
