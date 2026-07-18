use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use secrecy::{ExposeSecret, SecretString};
use sha2::{Digest, Sha256};
use time::OffsetDateTime;
use url::Url;

use super::InviteError;

const TOKEN_BYTES: usize = 32;
const INVITE_ID_HEX_CHARS: usize = 24;

pub struct InviteCreation {
    pub(super) token: SecretString,
    pub(super) id: String,
    pub(super) expires_at: OffsetDateTime,
}

#[derive(Clone)]
pub struct InviteReservation {
    pub(super) token_hash: String,
    pub(super) reservation_hash: String,
}

pub struct AllowedIdentity {
    invite_id: String,
    enrolled_at: OffsetDateTime,
}

impl InviteCreation {
    pub fn token(&self) -> &SecretString {
        &self.token
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn expires_at(&self) -> OffsetDateTime {
        self.expires_at
    }
}

impl AllowedIdentity {
    pub(super) fn new(invite_id: String, enrolled_at: OffsetDateTime) -> Self {
        Self {
            invite_id,
            enrolled_at,
        }
    }

    pub fn invite_id(&self) -> &str {
        &self.invite_id
    }

    pub fn enrolled_at(&self) -> OffsetDateTime {
        self.enrolled_at
    }
}

pub fn invitation_url(origin: &str, token: &SecretString) -> Result<Url, url::ParseError> {
    let mut url = Url::parse(origin)?;
    url.set_path("/login");
    url.set_query(None);
    url.set_fragment(Some(&format!("invite={}", token.expose_secret())));
    Ok(url)
}

pub(super) fn random_token() -> Result<SecretString, InviteError> {
    let mut bytes = [0_u8; TOKEN_BYTES];
    getrandom::fill(&mut bytes).map_err(|_| InviteError::Random)?;
    Ok(SecretString::from(URL_SAFE_NO_PAD.encode(bytes)))
}

pub(super) fn validate_token(token: &str) -> Result<(), InviteError> {
    let bytes = URL_SAFE_NO_PAD
        .decode(token)
        .map_err(|_| InviteError::Invalid)?;
    (bytes.len() == TOKEN_BYTES)
        .then_some(())
        .ok_or(InviteError::Invalid)
}

pub(super) fn hash_token(token: &str) -> String {
    Sha256::digest(token.as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

pub(super) fn invite_id(token_hash: &str) -> String {
    token_hash.chars().take(INVITE_ID_HEX_CHARS).collect()
}
