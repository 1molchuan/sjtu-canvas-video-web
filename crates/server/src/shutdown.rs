pub async fn wait_for_signal() {
    #[cfg(unix)]
    wait_for_unix_signal().await;
    #[cfg(not(unix))]
    wait_for_ctrl_c().await;
}

async fn wait_for_ctrl_c() {
    if tokio::signal::ctrl_c().await.is_err() {
        std::future::pending::<()>().await;
    }
}

#[cfg(unix)]
async fn wait_for_unix_signal() {
    use tokio::signal::unix::{SignalKind, signal};

    let Ok(mut terminate) = signal(SignalKind::terminate()) else {
        wait_for_ctrl_c().await;
        return;
    };
    tokio::select! {
        _ = wait_for_ctrl_c() => {},
        _ = terminate.recv() => {},
    }
}
