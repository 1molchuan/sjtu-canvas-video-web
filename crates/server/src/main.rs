use std::{env, error::Error, future::IntoFuture, io, net::SocketAddr, sync::Arc, time::Duration};

use canvas_core::client::ProtocolConfig;
use server::{
    app_router, auth::login::ProductionLoginProvider, config::AppConfig, gate, shutdown,
    state::AppState,
};
use tokio::net::TcpListener;
use tokio_util::sync::CancellationToken;
use tracing::{info, warn};
use tracing_subscriber::EnvFilter;

const CONFIG_ENV: &str = "SJTU_CANVAS_CONFIG";

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
    let protocol_config =
        ProtocolConfig::production(Duration::from_secs(config.server.api_timeout_seconds))
            .with_connect_timeout(Duration::from_secs(
                config.server.upstream_connect_timeout_seconds,
            ));
    let state = AppState::new(config, protocol_config, Arc::new(ProductionLoginProvider))?;
    let listener = TcpListener::bind(listen_addr).await?;

    info!(%listen_addr, "server listening");
    serve(listener, state, grace).await?;
    Ok(())
}

async fn serve(listener: TcpListener, state: AppState, grace: Duration) -> io::Result<()> {
    let stop_accepting = CancellationToken::new();
    let cleanup = state.spawn_cleanup_task();
    let service = app_router(state.clone()).into_make_service_with_connect_info::<SocketAddr>();
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
