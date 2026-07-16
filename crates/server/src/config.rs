use std::{
    fs,
    net::{IpAddr, SocketAddr},
    path::Path,
};

use serde::Deserialize;
use thiserror::Error;

#[derive(Debug, Clone, Deserialize)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub auth: AuthConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub session_ttl_hours: u64,
    pub pending_login_ttl_minutes: u64,
    pub download_ticket_ttl_seconds: u64,
    pub max_global_downloads: usize,
    pub max_downloads_per_user: usize,
    pub max_pending_logins: usize,
    pub api_timeout_seconds: u64,
    pub upstream_connect_timeout_seconds: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AuthConfig {
    pub allowed_users: Vec<String>,
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("failed to read configuration: {0}")]
    Read(#[from] std::io::Error),
    #[error("invalid TOML configuration: {0}")]
    Parse(#[from] toml::de::Error),
    #[error("server.host must be a literal IP address: {0}")]
    InvalidHost(String),
    #[error("public listen address is forbidden: {0}")]
    PublicBindForbidden(IpAddr),
    #[error("download limits must be positive and per-user must not exceed global")]
    InvalidDownloadLimits,
    #[error("time-to-live and timeout values must be positive")]
    InvalidDurations,
    #[error("max_pending_logins must be positive")]
    InvalidPendingLimit,
    #[error("auth.allowed_users must contain at least one stable identifier")]
    EmptyAllowlist,
}

impl AppConfig {
    pub fn load(path: impl AsRef<Path>) -> Result<Self, ConfigError> {
        let content = fs::read_to_string(path)?;
        let config: Self = toml::from_str(&content)?;
        config.validate()?;
        Ok(config)
    }

    pub fn listen_addr(&self) -> Result<SocketAddr, ConfigError> {
        let host = self.parse_host()?;
        Ok(SocketAddr::new(host, self.server.port))
    }

    fn validate(&self) -> Result<(), ConfigError> {
        self.parse_host()?;
        self.validate_limits()?;
        if self.auth.allowed_users.is_empty() {
            return Err(ConfigError::EmptyAllowlist);
        }
        Ok(())
    }

    fn parse_host(&self) -> Result<IpAddr, ConfigError> {
        let host = self
            .server
            .host
            .parse::<IpAddr>()
            .map_err(|_| ConfigError::InvalidHost(self.server.host.clone()))?;
        if !host.is_loopback() {
            return Err(ConfigError::PublicBindForbidden(host));
        }
        Ok(host)
    }

    fn validate_limits(&self) -> Result<(), ConfigError> {
        let server = &self.server;
        let downloads_valid = server.max_global_downloads > 0
            && server.max_downloads_per_user > 0
            && server.max_downloads_per_user <= server.max_global_downloads;
        if !downloads_valid {
            return Err(ConfigError::InvalidDownloadLimits);
        }
        if server.max_pending_logins == 0 {
            return Err(ConfigError::InvalidPendingLimit);
        }
        if !server.has_positive_durations() {
            return Err(ConfigError::InvalidDurations);
        }
        Ok(())
    }
}

impl ServerConfig {
    fn has_positive_durations(&self) -> bool {
        self.session_ttl_hours > 0
            && self.pending_login_ttl_minutes > 0
            && self.download_ticket_ttl_seconds > 0
            && self.api_timeout_seconds > 0
            && self.upstream_connect_timeout_seconds > 0
    }
}

#[cfg(test)]
mod tests {
    use super::{AppConfig, ConfigError};

    const CONFIG_TEMPLATE: &str = r#"
[server]
host = "{host}"
port = 3000
session_ttl_hours = 8
pending_login_ttl_minutes = 5
download_ticket_ttl_seconds = 60
max_global_downloads = 4
max_downloads_per_user = 1
max_pending_logins = 20
api_timeout_seconds = 30
upstream_connect_timeout_seconds = 10

[auth]
allowed_users = ["test-stable-id"]
"#;

    #[test]
    fn accepts_loopback_bind_address() {
        let text = CONFIG_TEMPLATE.replace("{host}", "127.0.0.1");
        let config: AppConfig = toml::from_str(&text).expect("configuration should parse");

        assert!(config.validate().is_ok());
    }

    #[test]
    fn rejects_public_bind_address() {
        let text = CONFIG_TEMPLATE.replace("{host}", "0.0.0.0");
        let config: AppConfig = toml::from_str(&text).expect("configuration should parse");

        assert!(matches!(
            config.validate(),
            Err(ConfigError::PublicBindForbidden(_))
        ));
    }
}
