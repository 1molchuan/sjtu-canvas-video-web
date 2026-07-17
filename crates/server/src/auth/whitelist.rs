use std::collections::HashSet;

use canvas_core::identity::{hash_stable_id as hash_identity, normalize_stable_id};
use secrecy::{ExposeSecret, SecretString};
use thiserror::Error;

use crate::config::AuthConfig;

const HASH_PREFIX: &str = "sha256:";

#[derive(Debug, Error, PartialEq, Eq)]
pub enum WhitelistError {
    #[error("stable identifier is empty")]
    EmptyStableId,
    #[error("stable identifier hash must use sha256 followed by 64 lowercase hex characters")]
    InvalidHash,
}

pub struct StableIdWhitelist {
    raw_ids: HashSet<String>,
    hashes: HashSet<String>,
}

impl StableIdWhitelist {
    pub fn from_config(config: &AuthConfig) -> Result<Self, WhitelistError> {
        let raw_ids = config
            .allowed_stable_ids
            .iter()
            .map(|value| normalize_stable_id(value).map_err(|_| WhitelistError::EmptyStableId))
            .collect::<Result<HashSet<_>, _>>()?;
        let hashes = config
            .allowed_stable_id_hashes
            .iter()
            .map(|value| validate_hash(value))
            .collect::<Result<HashSet<_>, _>>()?;
        Ok(Self { raw_ids, hashes })
    }

    pub fn allows(&self, stable_id: &SecretString) -> bool {
        let Ok(normalized) = normalize_stable_id(stable_id.expose_secret()) else {
            return false;
        };
        if self.raw_ids.contains(&normalized) {
            return true;
        }
        hash_normalized(&normalized).is_some_and(|hash| self.hashes.contains(&hash))
    }
}

pub fn hash_stable_id(value: &str) -> Result<String, WhitelistError> {
    hash_identity(value).map_err(|_| WhitelistError::EmptyStableId)
}

fn hash_normalized(value: &str) -> Option<String> {
    hash_identity(value).ok()
}

fn validate_hash(value: &str) -> Result<String, WhitelistError> {
    let Some(hex) = value.strip_prefix(HASH_PREFIX) else {
        return Err(WhitelistError::InvalidHash);
    };
    let valid = hex.len() == 64
        && hex
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte));
    valid
        .then(|| value.to_owned())
        .ok_or(WhitelistError::InvalidHash)
}
