use std::{env, error::Error};

use server::{app_router, config::AppConfig};
use tokio::net::TcpListener;
use tracing::info;
use tracing_subscriber::EnvFilter;

const CONFIG_ENV: &str = "SJTU_CANVAS_CONFIG";

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let config_path = env::var(CONFIG_ENV)?;
    let config = AppConfig::load(config_path)?;
    let listen_addr = config.listen_addr()?;
    let listener = TcpListener::bind(listen_addr).await?;

    info!(%listen_addr, "server listening");
    axum::serve(listener, app_router()).await?;
    Ok(())
}
