use std::sync::Arc;

use dashmap::DashMap;
use thiserror::Error;
use time::OffsetDateTime;

use super::{DownloadTicket, DownloadTicketId, TicketRequest};
use crate::{id::RandomIdError, session::SessionId};

#[derive(Default)]
pub struct DownloadTicketStore {
    entries: DashMap<DownloadTicketId, Arc<DownloadTicket>>,
}

#[derive(Debug, Error)]
pub enum TicketStoreError {
    #[error(transparent)]
    Random(#[from] RandomIdError),
    #[error("ticket resource binding is inconsistent")]
    BindingMismatch,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum TicketLookupError {
    #[error("download ticket is invalid")]
    Invalid,
    #[error("download ticket has expired")]
    Expired,
    #[error("download ticket belongs to another session")]
    SessionMismatch,
}

impl DownloadTicketStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn issue(
        &self,
        request: TicketRequest,
        now: OffsetDateTime,
        ttl: time::Duration,
    ) -> Result<Arc<DownloadTicket>, TicketStoreError> {
        validate_request(&request)?;
        let id = DownloadTicketId::generate()?;
        let ticket = Arc::new(DownloadTicket::new(id, request, now, now + ttl));
        self.entries.insert(ticket.id().clone(), ticket.clone());
        Ok(ticket)
    }

    pub fn resolve(
        &self,
        id: &DownloadTicketId,
        session_id: &SessionId,
        now: OffsetDateTime,
    ) -> Result<Arc<DownloadTicket>, TicketLookupError> {
        let ticket = self
            .entries
            .get(id)
            .map(|entry| entry.value().clone())
            .ok_or(TicketLookupError::Invalid)?;
        if ticket.expires_at() <= now {
            self.entries.remove(id);
            return Err(TicketLookupError::Expired);
        }
        if ticket.session_id() != session_id {
            return Err(TicketLookupError::SessionMismatch);
        }
        Ok(ticket)
    }

    pub fn remove_for_session(&self, session_id: &SessionId) -> usize {
        let ids = self
            .entries
            .iter()
            .filter(|entry| entry.value().session_id() == session_id)
            .map(|entry| entry.key().clone())
            .collect::<Vec<_>>();
        for id in &ids {
            self.entries.remove(id);
        }
        ids.len()
    }

    pub fn cleanup_expired(&self, now: OffsetDateTime) -> usize {
        let ids = self
            .entries
            .iter()
            .filter(|entry| entry.value().expires_at() <= now)
            .map(|entry| entry.key().clone())
            .collect::<Vec<_>>();
        for id in &ids {
            self.entries.remove(id);
        }
        ids.len()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn clear(&self) -> usize {
        let count = self.entries.len();
        self.entries.clear();
        count
    }
}

fn validate_request(request: &TicketRequest) -> Result<(), TicketStoreError> {
    let matches = request.record.course == request.parent.course
        && request.record.video == request.parent.video;
    matches
        .then_some(())
        .ok_or(TicketStoreError::BindingMismatch)
}
