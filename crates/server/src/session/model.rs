use std::sync::Arc;

use canvas_core::{canvas::UserIdentity, client::ProtocolContext};
use secrecy::SecretString;
use time::OffsetDateTime;
use tokio::sync::{OwnedSemaphorePermit, Semaphore};
use tokio_util::sync::CancellationToken;

use super::{SessionId, SessionResources};
use crate::id::RandomIdError;

pub struct UserSession {
    id: SessionId,
    csrf_secret: SecretString,
    identity: UserIdentity,
    protocol: ProtocolContext,
    created_at: OffsetDateTime,
    expires_at: OffsetDateTime,
    download_semaphore: Arc<Semaphore>,
    protocol_semaphore: Arc<Semaphore>,
    revocation: CancellationToken,
    resources: SessionResources,
}

#[derive(Clone, Copy)]
pub struct UserSessionOptions {
    pub expires_at: OffsetDateTime,
    pub max_downloads: usize,
}

impl UserSession {
    pub fn new(
        identity: UserIdentity,
        protocol: ProtocolContext,
        options: UserSessionOptions,
    ) -> Result<Self, RandomIdError> {
        let id = SessionId::generate()?;
        let csrf_secret = SecretString::from(SessionId::generate()?.expose().to_owned());
        Ok(Self {
            id,
            csrf_secret,
            identity,
            protocol,
            created_at: OffsetDateTime::now_utc(),
            expires_at: options.expires_at,
            download_semaphore: Arc::new(Semaphore::new(options.max_downloads)),
            protocol_semaphore: Arc::new(Semaphore::new(1)),
            revocation: CancellationToken::new(),
            resources: SessionResources::new(),
        })
    }

    pub fn id(&self) -> &SessionId {
        &self.id
    }

    pub fn protocol(&self) -> &ProtocolContext {
        &self.protocol
    }

    pub fn identity(&self) -> &UserIdentity {
        &self.identity
    }

    pub fn expires_at(&self) -> OffsetDateTime {
        self.expires_at
    }

    pub fn created_at(&self) -> OffsetDateTime {
        self.created_at
    }

    pub(crate) fn csrf_secret(&self) -> &SecretString {
        &self.csrf_secret
    }

    pub(crate) fn download_semaphore(&self) -> Arc<Semaphore> {
        self.download_semaphore.clone()
    }

    pub async fn protocol_permit(&self) -> Option<OwnedSemaphorePermit> {
        let permit = self.protocol_semaphore.clone().acquire_owned();
        tokio::select! {
            permit = permit => permit.ok(),
            _ = self.revocation.cancelled() => None,
        }
    }

    pub fn revoke(&self) {
        self.revocation.cancel();
    }

    pub fn is_revoked(&self) -> bool {
        self.revocation.is_cancelled()
    }

    pub(crate) fn revocation(&self) -> CancellationToken {
        self.revocation.clone()
    }

    pub fn resources(&self) -> &SessionResources {
        &self.resources
    }
}
