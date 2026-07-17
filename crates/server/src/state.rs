use std::{sync::Arc, time::Instant};

use canvas_core::client::ProtocolConfig;
use thiserror::Error;
use time::OffsetDateTime;
use tokio::sync::Semaphore;
use tokio_util::sync::CancellationToken;
use url::Url;

use crate::{
    auth::{
        login::LoginProvider,
        pending::PendingLoginStore,
        rate_limit::LoginRateLimiter,
        whitelist::{StableIdWhitelist, WhitelistError},
    },
    config::{AppConfig, ConfigError},
    gateway::{ProductionProtocolGateway, ProtocolGateway},
    session::SessionStore,
    ticket::DownloadTicketStore,
};

#[derive(Clone)]
pub struct AppState {
    inner: Arc<AppStateInner>,
}

struct AppStateInner {
    config: Arc<AppConfig>,
    public_origin: Url,
    protocol_config: ProtocolConfig,
    login_provider: Arc<dyn LoginProvider>,
    protocol_gateway: Arc<dyn ProtocolGateway>,
    whitelist: StableIdWhitelist,
    sessions: SessionStore,
    pending_logins: PendingLoginStore,
    login_rate_limiter: LoginRateLimiter,
    tickets: DownloadTicketStore,
    global_download_semaphore: Arc<Semaphore>,
    shutdown: CancellationToken,
}

#[derive(Debug, Error)]
pub enum StateError {
    #[error(transparent)]
    Config(#[from] ConfigError),
    #[error(transparent)]
    Whitelist(#[from] WhitelistError),
    #[error("validated public origin could not be parsed")]
    PublicOrigin,
}

pub struct AppServices {
    pub login_provider: Arc<dyn LoginProvider>,
    pub protocol_gateway: Arc<dyn ProtocolGateway>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CleanupSummary {
    pub sessions: usize,
    pub pending_logins: usize,
    pub tickets: usize,
}

impl AppState {
    pub fn new(
        config: AppConfig,
        protocol_config: ProtocolConfig,
        login_provider: Arc<dyn LoginProvider>,
    ) -> Result<Self, StateError> {
        Self::with_services(
            config,
            protocol_config,
            AppServices {
                login_provider,
                protocol_gateway: Arc::new(ProductionProtocolGateway),
            },
        )
    }

    pub fn with_services(
        config: AppConfig,
        protocol_config: ProtocolConfig,
        services: AppServices,
    ) -> Result<Self, StateError> {
        config.validate()?;
        let public_origin =
            Url::parse(&config.server.public_origin).map_err(|_| StateError::PublicOrigin)?;
        let whitelist = StableIdWhitelist::from_config(&config.auth)?;
        let pending_logins = PendingLoginStore::new(config.server.max_pending_logins);
        let login_rate_limiter =
            LoginRateLimiter::per_minute(config.server.max_qr_starts_per_minute);
        let global_download_semaphore =
            Arc::new(Semaphore::new(config.server.max_global_downloads));
        let inner = AppStateInner {
            config: Arc::new(config),
            public_origin,
            protocol_config,
            login_provider: services.login_provider,
            protocol_gateway: services.protocol_gateway,
            whitelist,
            sessions: SessionStore::new(),
            pending_logins,
            login_rate_limiter,
            tickets: DownloadTicketStore::new(),
            global_download_semaphore,
            shutdown: CancellationToken::new(),
        };
        Ok(Self {
            inner: Arc::new(inner),
        })
    }

    pub(crate) fn config(&self) -> &AppConfig {
        &self.inner.config
    }
    pub(crate) fn public_origin(&self) -> &Url {
        &self.inner.public_origin
    }
    pub(crate) fn protocol_config(&self) -> ProtocolConfig {
        self.inner.protocol_config.clone()
    }
    pub(crate) fn login_provider(&self) -> Arc<dyn LoginProvider> {
        self.inner.login_provider.clone()
    }
    pub(crate) fn protocol_gateway(&self) -> Arc<dyn ProtocolGateway> {
        self.inner.protocol_gateway.clone()
    }
    pub(crate) fn whitelist(&self) -> &StableIdWhitelist {
        &self.inner.whitelist
    }
    pub(crate) fn sessions(&self) -> &SessionStore {
        &self.inner.sessions
    }
    pub(crate) fn pending_logins(&self) -> &PendingLoginStore {
        &self.inner.pending_logins
    }
    pub(crate) fn login_rate_limiter(&self) -> &LoginRateLimiter {
        &self.inner.login_rate_limiter
    }
    pub(crate) fn tickets(&self) -> &DownloadTicketStore {
        &self.inner.tickets
    }
    pub(crate) fn global_download_semaphore(&self) -> Arc<Semaphore> {
        self.inner.global_download_semaphore.clone()
    }
    pub(crate) fn shutdown(&self) -> CancellationToken {
        self.inner.shutdown.clone()
    }

    pub fn cleanup_expired(&self, now: OffsetDateTime) -> CleanupSummary {
        CleanupSummary {
            sessions: self.inner.sessions.cleanup_expired(now),
            pending_logins: self.inner.pending_logins.expire(now),
            tickets: self.inner.tickets.cleanup_expired(now),
        }
    }

    pub fn begin_shutdown(&self) {
        self.inner.shutdown.cancel();
        self.inner.pending_logins.cancel_all();
        self.inner.sessions.revoke_all();
        self.inner.tickets.clear();
    }

    pub fn spawn_cleanup_task(&self) -> tokio::task::JoinHandle<()> {
        let state = self.clone();
        tokio::spawn(async move { state.cleanup_loop().await })
    }

    async fn cleanup_loop(self) {
        const CLEANUP_INTERVAL_SECONDS: u64 = 30;
        let mut interval =
            tokio::time::interval(std::time::Duration::from_secs(CLEANUP_INTERVAL_SECONDS));
        loop {
            tokio::select! {
                _ = self.inner.shutdown.cancelled() => break,
                _ = interval.tick() => {
                    self.cleanup_expired(OffsetDateTime::now_utc());
                    self.inner.login_rate_limiter.cleanup(Instant::now());
                }
            }
        }
    }
}
