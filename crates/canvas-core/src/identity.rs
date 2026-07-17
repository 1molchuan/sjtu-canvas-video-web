use sha2::{Digest, Sha256};
use thiserror::Error;
use unicode_normalization::UnicodeNormalization;

const HASH_PREFIX: &str = "sha256:";

#[derive(Debug, Clone, Copy, Error, PartialEq, Eq)]
pub enum StableIdentityError {
    #[error("stable identifier is empty")]
    Empty,
}

pub fn normalize_stable_id(value: &str) -> Result<String, StableIdentityError> {
    let normalized = value.trim().nfc().collect::<String>();
    if normalized.is_empty() {
        return Err(StableIdentityError::Empty);
    }
    Ok(normalized)
}

pub fn hash_stable_id(value: &str) -> Result<String, StableIdentityError> {
    let normalized = normalize_stable_id(value)?;
    let digest = Sha256::digest(normalized.as_bytes());
    let hex = digest
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    Ok(format!("{HASH_PREFIX}{hex}"))
}
