use axum::http::{HeaderMap, header};

use super::pending::{BrowserBinding, PendingLogin, PendingLoginId};
use crate::{
    config::{AppConfig, SameSite as ConfigSameSite},
    middleware::cookies::{self, CookieOptions, SameSite},
    session::SessionId,
};

pub struct PendingCookie {
    pub id: PendingLoginId,
    pub binding: BrowserBinding,
}

pub fn pending_name(config: &AppConfig) -> String {
    format!("{}_pending", config.cookie.name)
}

pub fn encode_pending(pending: &PendingLogin) -> String {
    format!(
        "{}.{}",
        pending.id().expose(),
        pending.browser_binding().expose()
    )
}

pub fn read_pending(headers: &HeaderMap, config: &AppConfig) -> Option<PendingCookie> {
    let name = pending_name(config);
    let raw = read_named(headers, &name)?;
    let (id, binding) = raw.split_once('.')?;
    Some(PendingCookie {
        id: PendingLoginId::parse(id)?,
        binding: BrowserBinding::parse(binding)?,
    })
}

pub fn read_session(headers: &HeaderMap, config: &AppConfig) -> Option<SessionId> {
    let raw = read_named(headers, &config.cookie.name)?;
    SessionId::parse(raw)
}

pub fn set_pending_cookie(config: &AppConfig, value: &str) -> String {
    let name = pending_name(config);
    let options = CookieOptions {
        name: &name,
        secure: config.cookie.secure,
        same_site: same_site(config.cookie.same_site),
        max_age_seconds: Some((config.server.pending_login_ttl_minutes * 60) as i64),
    };
    cookies::set_cookie(value, &options)
}

pub fn clear_pending_cookie(config: &AppConfig) -> String {
    let name = pending_name(config);
    let options = CookieOptions {
        name: &name,
        secure: config.cookie.secure,
        same_site: same_site(config.cookie.same_site),
        max_age_seconds: None,
    };
    cookies::clear_cookie(&options)
}

pub fn session_options(config: &AppConfig) -> CookieOptions<'_> {
    CookieOptions {
        name: &config.cookie.name,
        secure: config.cookie.secure,
        same_site: same_site(config.cookie.same_site),
        max_age_seconds: Some((config.server.session_ttl_hours * 60 * 60) as i64),
    }
}

fn read_named<'a>(headers: &'a HeaderMap, name: &str) -> Option<&'a str> {
    headers
        .get(header::COOKIE)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| cookies::read_cookie(value, name))
}

fn same_site(value: ConfigSameSite) -> SameSite {
    match value {
        ConfigSameSite::Lax => SameSite::Lax,
        ConfigSameSite::Strict => SameSite::Strict,
    }
}
