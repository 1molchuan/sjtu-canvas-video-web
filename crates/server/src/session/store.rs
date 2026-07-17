use std::sync::Arc;

use dashmap::DashMap;
use time::OffsetDateTime;

use super::{SessionId, UserSession};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionLookupError {
    Invalid,
    Expired,
    Revoked,
}

#[derive(Default)]
pub struct SessionStore {
    entries: DashMap<SessionId, Arc<UserSession>>,
}

impl SessionStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&self, session: Arc<UserSession>) {
        self.entries.insert(session.id().clone(), session);
    }

    pub fn get_active(&self, id: &SessionId, now: OffsetDateTime) -> Option<Arc<UserSession>> {
        self.lookup(id, now).ok()
    }

    pub fn lookup(
        &self,
        id: &SessionId,
        now: OffsetDateTime,
    ) -> Result<Arc<UserSession>, SessionLookupError> {
        let session = self
            .entries
            .get(id)
            .map(|entry| entry.value().clone())
            .ok_or(SessionLookupError::Invalid)?;
        if session.is_revoked() {
            self.remove(id);
            return Err(SessionLookupError::Revoked);
        }
        if session.expires_at() <= now {
            self.remove(id);
            return Err(SessionLookupError::Expired);
        }
        Ok(session)
    }

    pub fn remove(&self, id: &SessionId) -> Option<Arc<UserSession>> {
        let session = self.entries.remove(id).map(|(_, session)| session)?;
        session.revoke();
        Some(session)
    }

    pub fn cleanup_expired(&self, now: OffsetDateTime) -> usize {
        let expired = self
            .entries
            .iter()
            .filter(|entry| entry.value().expires_at() <= now)
            .map(|entry| entry.key().clone())
            .collect::<Vec<_>>();
        let count = expired.len();
        for id in expired {
            self.remove(&id);
        }
        count
    }

    pub fn revoke_all(&self) -> usize {
        let ids = self
            .entries
            .iter()
            .map(|entry| entry.key().clone())
            .collect::<Vec<_>>();
        for id in &ids {
            self.remove(id);
        }
        ids.len()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}
