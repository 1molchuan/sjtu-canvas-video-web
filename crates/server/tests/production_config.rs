use server::config::{AppConfig, ConfigError, DeploymentMode};
use std::{fs, path::PathBuf};

const VALID_HASH: &str = "sha256:1111111111111111111111111111111111111111111111111111111111111111";
#[cfg(windows)]
const FRONTEND_DIST: &str = "C:/srv/canvas-video/frontend/dist";
#[cfg(not(windows))]
const FRONTEND_DIST: &str = "/srv/canvas-video/frontend/dist";
#[cfg(windows)]
const INVITE_DB: &str = "C:/srv/canvas-video/private/invites.sqlite3";
#[cfg(not(windows))]
const INVITE_DB: &str = "/srv/canvas-video/private/invites.sqlite3";

fn production_config(overrides: &[(&str, &str)]) -> AppConfig {
    let mut text = format!(
        r#"
[server]
mode = "production"
host = "127.0.0.1"
port = 3100
public_origin = "https://canvas-video.example.test"
frontend_dist = "{FRONTEND_DIST}"
session_ttl_hours = 8
pending_login_ttl_minutes = 5
download_ticket_ttl_seconds = 60
max_global_downloads = 4
max_downloads_per_user = 1
max_pending_logins = 20
api_timeout_seconds = 30
upstream_connect_timeout_seconds = 10
shutdown_grace_seconds = 15

[auth]
allowed_stable_id_hashes = ["{VALID_HASH}"]

[invites]
database_path = "{INVITE_DB}"
default_ttl_hours = 24

[cookie]
name = "__Host-sjtu_canvas_video_session"
secure = true
same_site = "Lax"

[security]
csrf_header = "X-CSRF-Token"
trust_proxy_headers = false
"#,
    );
    for (from, to) in overrides {
        text = text.replace(from, to);
    }
    toml::from_str(&text).expect("production config should parse")
}

#[test]
fn accepts_explicit_safe_production_configuration() {
    let config = production_config(&[]);
    assert_eq!(config.server.mode, DeploymentMode::Production);
    assert!(config.validate().is_ok());
}

#[test]
fn production_requires_frontend_https_and_secure_host_cookie() {
    let cases: Vec<(Vec<(&str, &str)>, ConfigError)> = vec![
        (
            vec![(frontend_setting(), "")],
            ConfigError::ProductionFrontendRequired,
        ),
        (
            vec![(FRONTEND_DIST, "frontend/dist")],
            ConfigError::ProductionFrontendPathAbsolute,
        ),
        (
            vec![("https://canvas-video.example.test", "http://127.0.0.1:3100")],
            ConfigError::ProductionHttpsRequired,
        ),
        (
            vec![("secure = true", "secure = false")],
            ConfigError::ProductionSecureCookieRequired,
        ),
        (
            vec![(
                "__Host-sjtu_canvas_video_session",
                "sjtu_canvas_video_session",
            )],
            ConfigError::ProductionHostCookieRequired,
        ),
    ];
    for (overrides, expected) in cases {
        let config = production_config(&overrides);
        let actual = config
            .validate()
            .expect_err("must reject unsafe production config");
        assert_eq!(
            std::mem::discriminant(&actual),
            std::mem::discriminant(&expected)
        );
    }
}

#[test]
fn production_rejects_example_allowlist_values() {
    let placeholder = "sha256:REPLACE_ME";
    let config = production_config(&[(VALID_HASH, placeholder)]);
    assert!(matches!(
        config.validate(),
        Err(ConfigError::ProductionPlaceholderAllowlist)
    ));
}

#[test]
fn production_requires_absolute_invite_database_path() {
    let config = production_config(&[(INVITE_DB, "private/invites.sqlite3")]);
    assert!(matches!(
        config.validate(),
        Err(ConfigError::ProductionInvitePathAbsolute)
    ));
}

#[test]
fn production_rejects_zero_invite_ttl() {
    let config = production_config(&[("default_ttl_hours = 24", "default_ttl_hours = 0")]);
    assert!(matches!(
        config.validate(),
        Err(ConfigError::InvalidDurations)
    ));
}

#[test]
fn development_mode_keeps_frontend_optional() {
    let config = production_config(&[
        ("mode = \"production\"", "mode = \"development\""),
        (frontend_setting(), ""),
        ("https://canvas-video.example.test", "http://127.0.0.1:3100"),
        ("secure = true", "secure = false"),
        (
            "__Host-sjtu_canvas_video_session",
            "sjtu_canvas_video_session",
        ),
    ]);
    assert_eq!(config.server.mode, DeploymentMode::Development);
    assert!(config.validate().is_ok());
}

#[test]
fn ubuntu_example_matches_the_production_schema() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../deploy/ubuntu/production.example.toml");
    let text = fs::read_to_string(path).expect("Ubuntu production example should be readable");
    let config: AppConfig = toml::from_str(&text).expect("Ubuntu production example should parse");
    assert_eq!(config.server.host, "127.0.0.1");
    assert_eq!(config.server.port, 3100);
    assert_eq!(config.server.public_origin, "https://canvas.1molchuan.top");
    assert_eq!(config.server.mode, DeploymentMode::Production);
    assert_eq!(
        config.auth.allowed_stable_id_hashes,
        vec!["sha256:REPLACE_ME"]
    );
}

fn frontend_setting() -> &'static str {
    #[cfg(windows)]
    {
        "frontend_dist = \"C:/srv/canvas-video/frontend/dist\"\n"
    }
    #[cfg(not(windows))]
    {
        "frontend_dist = \"/srv/canvas-video/frontend/dist\"\n"
    }
}
