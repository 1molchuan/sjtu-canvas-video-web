use std::{
    fs,
    net::{IpAddr, SocketAddr},
    path::Path,
};

use serde::Deserialize;
use thiserror::Error;
use url::{Host, Url};

#[derive(Clone, Deserialize)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub auth: AuthConfig,
    pub cookie: CookieConfig,
    pub security: SecurityConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub public_origin: String,
    pub shutdown_grace_seconds: u64,
    pub session_ttl_hours: u64,
    pub pending_login_ttl_minutes: u64,
    pub download_ticket_ttl_seconds: u64,
    pub max_global_downloads: usize,
    pub max_downloads_per_user: usize,
    pub max_pending_logins: usize,
    #[serde(default = "default_max_qr_starts_per_minute")]
    pub max_qr_starts_per_minute: usize,
    pub api_timeout_seconds: u64,
    pub upstream_connect_timeout_seconds: u64,
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

    pub fn validate(&self) -> Result<(), ConfigError> {
        self.parse_host()?;
        self.validate_public_origin()?;
        self.validate_cookie()?;
        self.validate_limits()?;
        if self.auth.allowed_stable_ids.is_empty() && self.auth.allowed_stable_id_hashes.is_empty()
        {
            return Err(ConfigError::EmptyAllowlist);
        }
        if !self.cookie.http_only || self.cookie.path != "/" || self.cookie.domain.is_some() {
            return Err(ConfigError::InvalidHostCookie);
        }
        if self.security.max_request_body_bytes == 0 {
            return Err(ConfigError::InvalidDurations);
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

    fn validate_public_origin(&self) -> Result<(), ConfigError> {
        let value = &self.server.public_origin;
        let origin =
            Url::parse(value).map_err(|_| ConfigError::InvalidPublicOrigin(value.clone()))?;
        let valid_shape = matches!(origin.scheme(), "http" | "https")
            && origin.host().is_some()
            && origin.username().is_empty()
            && origin.password().is_none()
            && origin.path() == "/"
            && origin.query().is_none()
            && origin.fragment().is_none();
        let valid_transport = origin.scheme() == "https" || is_loopback_origin(&origin);
        if !valid_shape || !valid_transport {
            return Err(ConfigError::InvalidPublicOrigin(value.clone()));
        }
        Ok(())
    }

    fn validate_cookie(&self) -> Result<(), ConfigError> {
        let cookie = &self.cookie;
        let invalid_host_cookie = cookie.name.starts_with("__Host-")
            && (!cookie.secure || cookie.path != "/" || cookie.domain.is_some());
        if invalid_host_cookie {
            return Err(ConfigError::InvalidHostCookie);
        }
        let origin = Url::parse(&self.server.public_origin)
            .map_err(|_| ConfigError::InvalidPublicOrigin(self.server.public_origin.clone()))?;
        if !cookie.secure && !is_loopback_origin(&origin) {
            return Err(ConfigError::InsecureCookie);
        }
        Ok(())
    }

    fn validate_limits(&self) -> Result<(), ConfigError> {
        let server = &self.server;
        let downloads_valid = server.max_global_downloads > 0
            && server.max_downloads_per_user > 0
            && server.max_downloads_per_user <= server.max_global_downloads;
        if !downloads_valid {
            return Err(ConfigError::InvalidDownloadLimits);
        }
        if server.max_pending_logins == 0 || server.max_qr_starts_per_minute == 0 {
            return Err(ConfigError::InvalidPendingLimit);
        }
        if !server.has_positive_durations() {
            return Err(ConfigError::InvalidDurations);
        }
        Ok(())
    }
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

fn is_loopback_origin(origin: &Url) -> bool {
    match origin.host() {
        Some(Host::Ipv4(address)) => address.is_loopback(),
        Some(Host::Ipv6(address)) => address.is_loopback(),
        Some(Host::Domain(domain)) => domain.eq_ignore_ascii_case("localhost"),
        None => false,
    }
}

impl ServerConfig {
    fn has_positive_durations(&self) -> bool {
        self.shutdown_grace_seconds > 0
            && self.session_ttl_hours > 0
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
