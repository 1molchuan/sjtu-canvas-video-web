mod validation;

use std::{
    fs,
    net::IpAddr,
    path::{Path, PathBuf},
};

use serde::Deserialize;
use thiserror::Error;

#[derive(Clone, Deserialize)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub auth: AuthConfig,
    pub cookie: CookieConfig,
    pub security: SecurityConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    #[serde(default = "default_deployment_mode")]
    pub mode: DeploymentMode,
    pub host: String,
    pub port: u16,
    pub public_origin: String,
    #[serde(default)]
    pub frontend_dist: Option<PathBuf>,
    pub shutdown_grace_seconds: u64,
    pub session_ttl_hours: u64,
    pub pending_login_ttl_minutes: u64,
    pub download_ticket_ttl_seconds: u64,
    #[serde(default)]
    pub download_delivery: DownloadDelivery,
    pub max_global_downloads: usize,
    pub max_downloads_per_user: usize,
    pub max_pending_logins: usize,
    #[serde(default = "default_max_qr_starts_per_minute")]
    pub max_qr_starts_per_minute: usize,
    pub api_timeout_seconds: u64,
    pub upstream_connect_timeout_seconds: u64,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum DeploymentMode {
    Development,
    Production,
}

#[derive(Debug, Default, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DownloadDelivery {
    #[default]
    Proxy,
    RedirectExperimental,
}

#[derive(Clone, Deserialize)]
pub struct AuthConfig {
    #[serde(default)]
    pub allowed_stable_ids: Vec<String>,
    #[serde(default)]
    pub allowed_stable_id_hashes: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CookieConfig {
    pub name: String,
    pub secure: bool,
    #[serde(default = "default_true")]
    pub http_only: bool,
    pub same_site: SameSite,
    #[serde(default = "default_cookie_path")]
    pub path: String,
    #[serde(default)]
    pub domain: Option<String>,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
pub enum SameSite {
    #[serde(rename = "Strict", alias = "strict")]
    Strict,
    #[serde(rename = "Lax", alias = "lax")]
    Lax,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SecurityConfig {
    pub csrf_header: String,
    #[serde(default)]
    pub trust_proxy_headers: bool,
    #[serde(default = "default_max_request_body_bytes")]
    pub max_request_body_bytes: usize,
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
    #[error("server.public_origin must be an HTTPS origin or an HTTP loopback origin: {0}")]
    InvalidPublicOrigin(String),
    #[error("__Host- cookies require Secure, Path=/, and no Domain attribute")]
    InvalidHostCookie,
    #[error("non-loopback public origins require Secure cookies")]
    InsecureCookie,
    #[error("download limits must be positive and per-user must not exceed global")]
    InvalidDownloadLimits,
    #[error("time-to-live and timeout values must be positive")]
    InvalidDurations,
    #[error("max_pending_logins must be positive")]
    InvalidPendingLimit,
    #[error("auth allowlist must contain at least one stable identifier")]
    EmptyAllowlist,
    #[error("production mode requires server.frontend_dist")]
    ProductionFrontendRequired,
    #[error("production server.frontend_dist must be an absolute path")]
    ProductionFrontendPathAbsolute,
    #[error("production mode requires an HTTPS public origin")]
    ProductionHttpsRequired,
    #[error("production mode requires Secure cookies")]
    ProductionSecureCookieRequired,
    #[error("production mode requires a __Host- session cookie")]
    ProductionHostCookieRequired,
    #[error("production mode rejects example allowlist values")]
    ProductionPlaceholderAllowlist,
}

impl AppConfig {
    pub fn load(path: impl AsRef<Path>) -> Result<Self, ConfigError> {
        let content = fs::read_to_string(path)?;
        let config: Self = toml::from_str(&content)?;
        config.validate()?;
        Ok(config)
    }
}

fn default_deployment_mode() -> DeploymentMode {
    DeploymentMode::Development
}

fn default_true() -> bool {
    true
}

fn default_cookie_path() -> String {
    "/".to_owned()
}

fn default_max_request_body_bytes() -> usize {
    64 * 1024
}

fn default_max_qr_starts_per_minute() -> usize {
    6
}

#[cfg(test)]
mod tests {
    use super::{AppConfig, ConfigError};

    const CONFIG_TEMPLATE: &str = r#"
[server]
host = "{host}"
port = 3000
public_origin = "https://video.example.test"
shutdown_grace_seconds = 15
session_ttl_hours = 8
pending_login_ttl_minutes = 5
download_ticket_ttl_seconds = 60
max_global_downloads = 4
max_downloads_per_user = 1
max_pending_logins = 20
api_timeout_seconds = 30
upstream_connect_timeout_seconds = 10

[auth]
allowed_stable_ids = ["fake-stable-user-001"]
allowed_stable_id_hashes = []

[cookie]
name = "__Host-sjtu-canvas-session"
secure = true
http_only = true
same_site = "Lax"
path = "/"

[security]
csrf_header = "x-csrf-token"
trust_proxy_headers = false
max_request_body_bytes = 65536
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

    #[test]
    fn rejects_insecure_cookie_for_https_origin() {
        let text = CONFIG_TEMPLATE
            .replace("{host}", "127.0.0.1")
            .replace("__Host-sjtu-canvas-session", "sjtu-canvas-session")
            .replace("secure = true", "secure = false");
        let config: AppConfig = toml::from_str(&text).expect("configuration should parse");

        assert!(config.validate().is_err());
    }
}
