use std::collections::VecDeque;

use parking_lot::Mutex;
use secrecy::{ExposeSecret, SecretString};
use serde::{Serialize, Serializer};
use subtle::ConstantTimeEq;
use thiserror::Error;
use time::OffsetDateTime;
use tokio::sync::broadcast;
use tokio_util::sync::CancellationToken;

use crate::id::{RandomIdError, opaque_id};
use crate::invite::InviteReservation;

use super::super::login::AuthenticatedLogin;

const EVENT_CHANNEL_CAPACITY: usize = 16;
const EVENT_HISTORY_CAPACITY: usize = 16;
const COMPLETED_RETENTION_SECONDS: i64 = 30;

opaque_id!(PendingLoginId);
opaque_id!(BrowserBinding);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PendingLoginState {
    Starting,
    WaitingForQr,
    WaitingForScan,
    Authenticating,
    Completed,
    Failed,
    Expired,
    Cancelled,
}

#[derive(Clone)]
pub struct BrowserQrUrl(SecretString);

impl BrowserQrUrl {
    pub fn new(value: SecretString) -> Self {
        Self(value)
    }
}

impl Serialize for BrowserQrUrl {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.0.expose_secret())
    }
}

impl PartialEq for BrowserQrUrl {
    fn eq(&self, other: &Self) -> bool {
        self.0.expose_secret() == other.0.expose_secret()
    }
}

impl Eq for BrowserQrUrl {}

impl std::fmt::Debug for BrowserQrUrl {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("BrowserQrUrl(<redacted>)")
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum LoginEvent {
    Started,
    Qr { url: BrowserQrUrl },
    Scanned,
    Authenticating,
    Authenticated,
    Rejected,
    Expired,
    Error { code: String, message: String },
}

impl LoginEvent {
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            Self::Authenticated | Self::Rejected | Self::Expired | Self::Error { .. }
        )
    }
}

struct PendingInner {
    state: PendingLoginState,
    events: VecDeque<LoginEvent>,
    authenticated: Option<AuthenticatedLogin>,
    completed_expires_at: Option<OffsetDateTime>,
    invite_reservation: Option<InviteReservation>,
}

pub struct PendingLogin {
    id: PendingLoginId,
    browser_binding: BrowserBinding,
    inner: Mutex<PendingInner>,
    created_at: OffsetDateTime,
    pub(super) expires_at: OffsetDateTime,
    event_tx: broadcast::Sender<LoginEvent>,
    cancellation: CancellationToken,
}

#[derive(Debug, Error)]
pub enum PendingStoreError {
    #[error("too many pending logins")]
    Capacity,
    #[error(transparent)]
    Random(#[from] RandomIdError),
    #[error("pending login state transition is invalid")]
    InvalidState,
}

impl PendingLogin {
    pub(super) fn new(
        created_at: OffsetDateTime,
        expires_at: OffsetDateTime,
        invite_reservation: Option<InviteReservation>,
    ) -> Result<Self, RandomIdError> {
        let (event_tx, _) = broadcast::channel(EVENT_CHANNEL_CAPACITY);
        Ok(Self {
            id: PendingLoginId::generate()?,
            browser_binding: BrowserBinding::generate()?,
            inner: Mutex::new(PendingInner {
                state: PendingLoginState::Starting,
                events: VecDeque::new(),
                authenticated: None,
                completed_expires_at: None,
                invite_reservation,
            }),
            created_at,
            expires_at,
            event_tx,
            cancellation: CancellationToken::new(),
        })
    }

    pub fn id(&self) -> &PendingLoginId {
        &self.id
    }

    pub fn browser_binding(&self) -> &BrowserBinding {
        &self.browser_binding
    }

    pub fn cancellation(&self) -> CancellationToken {
        self.cancellation.clone()
    }

    pub fn subscribe(&self) -> broadcast::Receiver<LoginEvent> {
        self.event_tx.subscribe()
    }

    pub fn created_at(&self) -> OffsetDateTime {
        self.created_at
    }

    pub async fn state(&self) -> PendingLoginState {
        self.inner.lock().state
    }

    pub async fn transition(&self, next: PendingLoginState) -> Result<(), PendingStoreError> {
        let mut inner = self.inner.lock();
        if !valid_transition(inner.state, next) {
            return Err(PendingStoreError::InvalidState);
        }
        inner.state = next;
        Ok(())
    }

    pub fn publish(&self, event: LoginEvent) {
        push_event(&mut self.inner.lock().events, event.clone());
        let _ = self.event_tx.send(event);
    }

    pub fn event_history(&self) -> Vec<LoginEvent> {
        self.inner.lock().events.iter().cloned().collect()
    }

    pub fn invite_reservation(&self) -> Option<InviteReservation> {
        self.inner.lock().invite_reservation.clone()
    }

    pub async fn complete(&self, login: AuthenticatedLogin) -> Result<(), PendingStoreError> {
        let mut inner = self.inner.lock();
        if !valid_transition(inner.state, PendingLoginState::Completed) {
            return Err(PendingStoreError::InvalidState);
        }
        inner.state = PendingLoginState::Completed;
        inner.authenticated = Some(login);
        inner.completed_expires_at =
            Some(OffsetDateTime::now_utc() + time::Duration::seconds(COMPLETED_RETENTION_SECONDS));
        push_event(&mut inner.events, LoginEvent::Authenticated);
        drop(inner);
        let _ = self.event_tx.send(LoginEvent::Authenticated);
        Ok(())
    }

    pub(super) fn expire(&self) {
        let mut inner = self.inner.lock();
        if matches!(
            inner.state,
            PendingLoginState::Expired | PendingLoginState::Cancelled
        ) {
            return;
        }
        inner.state = PendingLoginState::Expired;
        inner.authenticated = None;
        inner.completed_expires_at = None;
        push_event(&mut inner.events, LoginEvent::Expired);
        drop(inner);
        self.cancellation.cancel();
        let _ = self.event_tx.send(LoginEvent::Expired);
    }

    pub(super) fn take_authenticated(
        &self,
        binding: &BrowserBinding,
        now: OffsetDateTime,
    ) -> Option<AuthenticatedLogin> {
        if now >= self.effective_expires_at() || !binding_matches(&self.browser_binding, binding) {
            return None;
        }
        let mut inner = self.inner.lock();
        (inner.state == PendingLoginState::Completed)
            .then(|| inner.authenticated.take())
            .flatten()
    }

    pub(super) fn cancel(&self) {
        let mut inner = self.inner.lock();
        inner.authenticated = None;
        inner.completed_expires_at = None;
        if !inner.state.is_terminal() {
            inner.state = PendingLoginState::Cancelled;
        }
        drop(inner);
        self.cancellation.cancel();
    }

    pub(super) fn effective_expires_at(&self) -> OffsetDateTime {
        self.inner
            .lock()
            .completed_expires_at
            .map_or(self.expires_at, |completed| completed.min(self.expires_at))
    }
}

impl PendingLoginState {
    pub fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::Completed | Self::Failed | Self::Expired | Self::Cancelled
        )
    }
}

pub(super) fn binding_matches(expected: &BrowserBinding, candidate: &BrowserBinding) -> bool {
    let left = expected.expose().as_bytes();
    let right = candidate.expose().as_bytes();
    left.len() == right.len() && bool::from(left.ct_eq(right))
}

fn push_event(events: &mut VecDeque<LoginEvent>, event: LoginEvent) {
    if events.len() == EVENT_HISTORY_CAPACITY {
        events.pop_front();
    }
    events.push_back(event);
}

fn valid_transition(current: PendingLoginState, next: PendingLoginState) -> bool {
    use PendingLoginState as S;
    matches!(
        (current, next),
        (
            S::Starting,
            S::WaitingForQr | S::Failed | S::Expired | S::Cancelled
        ) | (
            S::WaitingForQr,
            S::WaitingForScan | S::Failed | S::Expired | S::Cancelled
        ) | (
            S::WaitingForScan,
            S::Authenticating | S::Failed | S::Expired | S::Cancelled
        ) | (
            S::Authenticating,
            S::Completed | S::Failed | S::Expired | S::Cancelled
        )
    )
}
