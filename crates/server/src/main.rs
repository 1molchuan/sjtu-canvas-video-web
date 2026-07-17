use std::{env, error::Error, future::IntoFuture, io, net::SocketAddr, sync::Arc, time::Duration};

use axum::Router;
use canvas_core::client::ProtocolConfig;
use server::{
    FrontendAssets, app_router, app_router_with_frontend, auth::login::ProductionLoginProvider,
    config::AppConfig, gate, shutdown, state::AppState,
};
use tokio::net::TcpListener;
use tokio_util::sync::CancellationToken;
use tracing::{info, warn};
use tracing_subscriber::EnvFilter;
use url::Url;

const CONFIG_ENV: &str = "SJTU_CANVAS_CONFIG";

struct ServerRuntime {
    app: Router,
    state: AppState,
    grace: Duration,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    gate::ensure_real_protocol_enabled()?;
    let config_path = env::var(CONFIG_ENV)?;
    let config = AppConfig::load(config_path)?;
    let listen_addr = config.listen_addr()?;
    let grace = Duration::from_secs(config.server.shutdown_grace_seconds);
    let frontend = config
        .server
        .frontend_dist
        .as_ref()
        .map(FrontendAssets::load)
        .transpose()?;
    let frontend_enabled = frontend.is_some();
    let public_origin_host = public_origin_host(&config)?;
    let allowlist_entries =
        config.auth.allowed_stable_ids.len() + config.auth.allowed_stable_id_hashes.len();
    let protocol_config =
        ProtocolConfig::production(Duration::from_secs(config.server.api_timeout_seconds))
            .with_connect_timeout(Duration::from_secs(
                config.server.upstream_connect_timeout_seconds,
            ));
    let state = AppState::new(config, protocol_config, Arc::new(ProductionLoginProvider))?;
    let app = match frontend {
        Some(assets) => app_router_with_frontend(state.clone(), assets),
        None => app_router(state.clone()),
    };
    let listener = TcpListener::bind(listen_addr).await?;

    info!(
        %listen_addr,
        %public_origin_host,
        frontend_enabled,
        allowlist_entries,
        "server listening"
    );
    serve(listener, ServerRuntime { app, state, grace }).await?;
    Ok(())
}

fn public_origin_host(config: &AppConfig) -> Result<String, io::Error> {
    let origin = Url::parse(&config.server.public_origin).map_err(io::Error::other)?;
    origin
        .host_str()
        .map(str::to_owned)
        .ok_or_else(|| io::Error::other("validated public origin has no host"))
}

async fn serve(listener: TcpListener, runtime: ServerRuntime) -> io::Result<()> {
    let ServerRuntime { app, state, grace } = runtime;
    let stop_accepting = CancellationToken::new();
    let cleanup = state.spawn_cleanup_task();
    let service = app.into_make_service_with_connect_info::<SocketAddr>();
    let server = axum::serve(listener, service)
        .with_graceful_shutdown(stop_accepting.clone().cancelled_owned())
        .into_future();
    tokio::pin!(server);
    tokio::select! {
        result = &mut server => result?,
        _ = shutdown::wait_for_signal() => {
            stop_accepting.cancel();
            state.begin_shutdown();
            if tokio::time::timeout(grace, &mut server).await.is_err() {
                warn!("graceful shutdown deadline reached");
            }
        }
    }
    state.begin_shutdown();
    let _ = cleanup.await;
    Ok(())
}
