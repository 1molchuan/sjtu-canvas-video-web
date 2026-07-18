use std::{path::PathBuf, process::ExitCode};

use clap::{Parser, Subcommand};
use server::{
    config::AppConfig,
    invite::{InviteStore, invitation_url},
};
use time::{Duration, OffsetDateTime, format_description::well_known::Rfc3339};

#[derive(Parser)]
#[command(about = "Manage private one-time invitation links")]
struct Args {
    #[arg(long)]
    config: PathBuf,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    Create {
        #[arg(long)]
        ttl_hours: Option<u64>,
    },
    List,
    Revoke {
        invite_id: String,
    },
}

fn main() -> ExitCode {
    match run(Args::parse()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("{message}");
            ExitCode::FAILURE
        }
    }
}

fn run(args: Args) -> Result<(), String> {
    let config = AppConfig::load(&args.config).map_err(|_| "无法读取有效配置。".to_owned())?;
    let path = config
        .invites
        .database_path
        .as_ref()
        .ok_or_else(|| "配置未启用邀请存储。".to_owned())?;
    let store = InviteStore::open(path).map_err(|_| "无法打开邀请存储。".to_owned())?;
    match args.command {
        Command::Create { ttl_hours } => create(&config, &store, ttl_hours),
        Command::List => list(&store),
        Command::Revoke { invite_id } => revoke(&store, &invite_id),
    }
}

fn create(config: &AppConfig, store: &InviteStore, ttl_hours: Option<u64>) -> Result<(), String> {
    let hours = ttl_hours.unwrap_or(config.invites.default_ttl_hours);
    let hours = i64::try_from(hours).map_err(|_| "邀请有效期超出范围。".to_owned())?;
    if hours <= 0 {
        return Err("邀请有效期必须大于零。".to_owned());
    }
    let invitation = store
        .create(OffsetDateTime::now_utc(), Duration::hours(hours))
        .map_err(|_| "无法创建邀请。".to_owned())?;
    let url = invitation_url(&config.server.public_origin, invitation.token())
        .map_err(|_| "无法生成邀请链接。".to_owned())?;
    let expires = invitation
        .expires_at()
        .format(&Rfc3339)
        .map_err(|_| "无法格式化邀请有效期。".to_owned())?;
    println!("invite_id={}", invitation.id());
    println!("expires_at={expires}");
    println!("url={}", url.as_str());
    Ok(())
}

fn list(store: &InviteStore) -> Result<(), String> {
    let identities = store
        .list_allowed()
        .map_err(|_| "无法读取已邀请用户。".to_owned())?;
    for identity in identities {
        let enrolled = identity
            .enrolled_at()
            .format(&Rfc3339)
            .map_err(|_| "无法格式化登记时间。".to_owned())?;
        println!("invite_id={} enrolled_at={enrolled}", identity.invite_id());
    }
    Ok(())
}

fn revoke(store: &InviteStore, invite_id: &str) -> Result<(), String> {
    if !valid_invite_id(invite_id) {
        return Err("邀请编号格式无效。".to_owned());
    }
    let revoked = store
        .revoke(invite_id)
        .map_err(|_| "无法撤销邀请用户。".to_owned())?;
    if !revoked {
        return Err("没有找到该邀请编号。".to_owned());
    }
    println!("revoked={invite_id}");
    Ok(())
}

fn valid_invite_id(value: &str) -> bool {
    value.len() == 24
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}
