use std::net::{IpAddr, SocketAddr};

use url::{Host, Url};

use super::{AppConfig, ConfigError, DeploymentMode, ServerConfig};

const HASH_PREFIX: &str = "sha256:";

impl AppConfig {
    pub fn listen_addr(&self) -> Result<SocketAddr, ConfigError> {
        let host = self.parse_host()?;
        Ok(SocketAddr::new(host, self.server.port))
    }

    pub fn validate(&self) -> Result<(), ConfigError> {
        self.parse_host()?;
        self.validate_public_origin()?;
        self.validate_production()?;
        self.validate_cookie()?;
        self.validate_limits()?;
        self.validate_allowlist()?;
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
        let origin = parse_origin(value)?;
        let valid_transport = origin.scheme() == "https" || is_loopback_origin(&origin);
        if !valid_transport {
            return Err(ConfigError::InvalidPublicOrigin(value.clone()));
        }
        Ok(())
    }

    fn validate_production(&self) -> Result<(), ConfigError> {
        if self.server.mode != DeploymentMode::Production {
            return Ok(());
        }
        let frontend_dist = self
            .server
            .frontend_dist
            .as_ref()
            .ok_or(ConfigError::ProductionFrontendRequired)?;
        if !frontend_dist.is_absolute() {
            return Err(ConfigError::ProductionFrontendPathAbsolute);
        }
        if parse_origin(&self.server.public_origin)?.scheme() != "https" {
            return Err(ConfigError::ProductionHttpsRequired);
        }
        if !self.cookie.secure {
            return Err(ConfigError::ProductionSecureCookieRequired);
        }
        if !self.cookie.name.starts_with("__Host-") {
            return Err(ConfigError::ProductionHostCookieRequired);
        }
        if self.has_placeholder_allowlist() {
            return Err(ConfigError::ProductionPlaceholderAllowlist);
        }
        Ok(())
    }

    fn validate_cookie(&self) -> Result<(), ConfigError> {
        let cookie = &self.cookie;
        let invalid_host_cookie = cookie.name.starts_with("__Host-")
            && (!cookie.secure || cookie.path != "/" || cookie.domain.is_some());
        if invalid_host_cookie || !cookie.http_only || cookie.path != "/" || cookie.domain.is_some()
        {
            return Err(ConfigError::InvalidHostCookie);
        }
        let origin = parse_origin(&self.server.public_origin)?;
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

    fn validate_allowlist(&self) -> Result<(), ConfigError> {
        if self.auth.allowed_stable_ids.is_empty() && self.auth.allowed_stable_id_hashes.is_empty()
        {
            return Err(ConfigError::EmptyAllowlist);
        }
        Ok(())
    }

    fn has_placeholder_allowlist(&self) -> bool {
        self.auth
            .allowed_stable_ids
            .iter()
            .chain(self.auth.allowed_stable_id_hashes.iter())
            .any(|value| is_placeholder(value))
    }
}

fn parse_origin(value: &str) -> Result<Url, ConfigError> {
    let origin =
        Url::parse(value).map_err(|_| ConfigError::InvalidPublicOrigin(value.to_owned()))?;
    let valid = matches!(origin.scheme(), "http" | "https")
        && origin.host().is_some()
        && origin.username().is_empty()
        && origin.password().is_none()
        && origin.path() == "/"
        && origin.query().is_none()
        && origin.fragment().is_none();
    if !valid {
        return Err(ConfigError::InvalidPublicOrigin(value.to_owned()));
    }
    Ok(origin)
}

fn is_loopback_origin(origin: &Url) -> bool {
    match origin.host() {
        Some(Host::Ipv4(address)) => address.is_loopback(),
        Some(Host::Ipv6(address)) => address.is_loopback(),
        Some(Host::Domain(domain)) => domain.eq_ignore_ascii_case("localhost"),
        None => false,
    }
}

fn is_placeholder(value: &str) -> bool {
    let normalized = value.trim();
    normalized.to_ascii_uppercase().contains("REPLACE_ME")
        || normalized.starts_with("fake-")
        || normalized
            .strip_prefix(HASH_PREFIX)
            .is_some_and(|hash| !hash.is_empty() && hash.bytes().all(|byte| byte == b'0'))
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
