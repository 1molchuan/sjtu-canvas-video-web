use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use dashmap::DashMap;
use time::OffsetDateTime;

use crate::auth::login::AuthenticatedLogin;

use super::model::{
    BrowserBinding, PendingLogin, PendingLoginId, PendingStoreError, binding_matches,
};

pub struct PendingLoginStore {
    entries: DashMap<PendingLoginId, Arc<PendingLogin>>,
    count: AtomicUsize,
    max_entries: usize,
}

impl PendingLoginStore {
    pub fn new(max_entries: usize) -> Self {
        Self {
            entries: DashMap::new(),
            count: AtomicUsize::new(0),
            max_entries,
        }
    }

    pub fn create(
        &self,
        now: OffsetDateTime,
        ttl: time::Duration,
    ) -> Result<Arc<PendingLogin>, PendingStoreError> {
        self.reserve_capacity()?;
        let pending = match PendingLogin::new(now, now + ttl) {
            Ok(pending) => Arc::new(pending),
            Err(error) => {
                self.release_capacity();
                return Err(error.into());
            }
        };
        self.entries.insert(pending.id().clone(), pending.clone());
        Ok(pending)
    }

    pub fn get_authorized(
        &self,
        id: &PendingLoginId,
        binding: &BrowserBinding,
        now: OffsetDateTime,
    ) -> Option<Arc<PendingLogin>> {
        let pending = self.entries.get(id).map(|entry| entry.value().clone())?;
        if pending.effective_expires_at() <= now {
            self.remove_expired(id);
            return None;
        }
        binding_matches(pending.browser_binding(), binding).then_some(pending)
    }

    pub fn authorize(
        &self,
        id: &PendingLoginId,
        binding: &BrowserBinding,
        now: OffsetDateTime,
    ) -> bool {
        self.get_authorized(id, binding, now).is_some()
    }

    pub fn claim_authenticated(
        &self,
        id: &PendingLoginId,
        binding: &BrowserBinding,
        now: OffsetDateTime,
    ) -> Option<AuthenticatedLogin> {
        let pending = self.get_authorized(id, binding, now)?;
        let login = pending.take_authenticated(binding, now)?;
        if self.entries.remove(id).is_some() {
            self.release_capacity();
        }
        Some(login)
    }

    pub fn expire(&self, now: OffsetDateTime) -> usize {
        let ids = self
            .entries
            .iter()
            .filter(|entry| entry.value().effective_expires_at() <= now)
            .map(|entry| entry.key().clone())
            .collect::<Vec<_>>();
        for id in &ids {
            self.remove_expired(id);
        }
        ids.len()
    }

    pub fn cancel_all(&self) -> usize {
        let ids = self
            .entries
            .iter()
            .map(|entry| entry.key().clone())
            .collect::<Vec<_>>();
        let mut removed = 0;
        for id in &ids {
            if let Some((_, pending)) = self.entries.remove(id) {
                pending.cancel();
                self.release_capacity();
                removed += 1;
            }
        }
        removed
    }

    pub fn len(&self) -> usize {
        self.count.load(Ordering::Acquire)
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn reserve_capacity(&self) -> Result<(), PendingStoreError> {
        self.count
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |count| {
                (count < self.max_entries).then_some(count + 1)
            })
            .map(|_| ())
            .map_err(|_| PendingStoreError::Capacity)
    }

    fn release_capacity(&self) {
        self.count.fetch_sub(1, Ordering::AcqRel);
    }

    fn remove_expired(&self, id: &PendingLoginId) {
        if let Some((_, pending)) = self.entries.remove(id) {
            pending.expire();
            self.release_capacity();
        }
    }
}
