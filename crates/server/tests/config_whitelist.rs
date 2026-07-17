use secrecy::SecretString;
use server::{
    auth::whitelist::{StableIdWhitelist, hash_stable_id},
    config::{AppConfig, ConfigError},
};
use std::path::PathBuf;

const BASE_CONFIG: &str = r#"
[server]
host = "127.0.0.1"
port = 3000
public_origin = "https://canvas-video.example.test"
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
allowed_stable_ids = ["Exact-Id", "café"]
allowed_stable_id_hashes = ["{hash}"]

[cookie]
name = "__Host-sjtu_canvas_video_session"
secure = true
same_site = "Lax"

[security]
csrf_header = "X-CSRF-Token"
trust_proxy_headers = false
"#;

fn config(hash: &str) -> AppConfig {
    toml::from_str(&BASE_CONFIG.replace("{hash}", hash)).expect("configuration should parse")
}

#[test]
fn whitelist_accepts_exact_nfc_and_hash_but_remains_case_sensitive() {
    let hashed = hash_stable_id("private-id").expect("hash should be generated");
    let config = config(&hashed);
    let whitelist = StableIdWhitelist::from_config(&config.auth)
        .expect("whitelist configuration should be valid");

    assert!(whitelist.allows(&SecretString::from("Exact-Id".to_owned())));
    assert!(!whitelist.allows(&SecretString::from("exact-id".to_owned())));
    assert!(whitelist.allows(&SecretString::from("cafe\u{301}".to_owned())));
    assert!(whitelist.allows(&SecretString::from("private-id".to_owned())));
}

#[test]
fn secure_host_cookie_and_public_origin_are_validated() {
    let hash = hash_stable_id("private-id").expect("hash should be generated");
    let valid = config(&hash);
    assert!(valid.validate().is_ok());

    let invalid = BASE_CONFIG
        .replace("{hash}", &hash)
        .replace("secure = true", "secure = false");
    let invalid: AppConfig = toml::from_str(&invalid).expect("configuration should parse");
    assert!(matches!(
        invalid.validate(),
        Err(ConfigError::InvalidHostCookie)
    ));
}

#[test]
fn repository_example_configuration_is_valid() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../config/example.toml");
    let config = AppConfig::load(path).expect("example configuration should be valid");
    assert_eq!(
        config.listen_addr().expect("address should parse").port(),
        3000
    );
}
