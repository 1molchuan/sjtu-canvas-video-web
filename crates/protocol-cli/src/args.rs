use clap::{Args, Parser, Subcommand};

const DEFAULT_TIMEOUT_SECONDS: u64 = 300;
const MAX_TIMEOUT_SECONDS: u64 = 900;

#[derive(Debug, Parser)]
#[command(name = "protocol-cli", about = "SJTU protocol validation CLI")]
pub struct Cli {
    #[arg(long, global = true)]
    pub debug: bool,

    #[arg(long, global = true)]
    pub json_output: bool,

    #[arg(
        long,
        global = true,
        default_value_t = DEFAULT_TIMEOUT_SECONDS,
        value_parser = clap::value_parser!(u64).range(1..=MAX_TIMEOUT_SECONDS)
    )]
    pub timeout_seconds: u64,

    #[arg(long, global = true)]
    pub no_course_discovery: bool,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    Login,
    DiscoverCourses,
    InspectCourse(CourseArgs),
    Full(CourseArgs),
}

#[derive(Debug, Clone, Args)]
pub struct CourseArgs {
    #[arg(long, value_parser = clap::value_parser!(i64).range(1..))]
    pub course_id: i64,

    #[arg(long)]
    pub video_id: Option<String>,

    #[arg(long)]
    pub probe_direct: bool,
}
