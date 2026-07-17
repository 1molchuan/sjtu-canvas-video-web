use std::{sync::Arc, time::Duration};

use canvas_core::{
    canvas::{IdentitySource, UserIdentity},
    client::{ProtocolConfig, ProtocolContext},
};
use secrecy::SecretString;
use server::{
    auth::login::AuthenticatedLogin,
    auth::pending::{LoginEvent, PendingLoginState, PendingLoginStore},
    session::{SessionLookupError, SessionStore, UserSession},
};
use time::OffsetDateTime;

fn protocol_context() -> ProtocolContext {
    ProtocolContext::new(ProtocolConfig::production(Duration::from_secs(1)))
        .expect("test protocol context should build")
}

fn identity(stable_id: &str) -> UserIdentity {
    UserIdentity {
        stable_id: SecretString::from(stable_id.to_owned()),
        account: None,
        display_name: None,
        source: IdentitySource::MySjtuAccount,
    }
}

fn user_session(stable_id: &str, expires_at: OffsetDateTime) -> Arc<UserSession> {
    Arc::new(
        UserSession::new(
            identity(stable_id),
            protocol_context(),
            server::session::UserSessionOptions {
                expires_at,
                max_downloads: 1,
            },
        )
        .expect("test session should build"),
    )
}

#[test]
fn session_store_rejects_expired_sessions_and_preserves_isolation() {
    let store = SessionStore::new();
    let now = OffsetDateTime::now_utc();
    let active = user_session("user-a", now + time::Duration::hours(1));
    let expired = user_session("user-b", now - time::Duration::seconds(1));
    let active_id = active.id().clone();
    let expired_id = expired.id().clone();

    store.insert(active.clone());
    store.insert(expired);

    assert!(Arc::ptr_eq(
        &store
            .get_active(&active_id, now)
            .expect("active session exists"),
        &active
    ));
    assert!(matches!(
        store.lookup(&expired_id, now),
        Err(SessionLookupError::Expired)
    ));
    assert_eq!(store.len(), 1);
    assert!(!Arc::ptr_eq(
        &active.protocol().cookie_store,
        &user_session("user-c", now + time::Duration::hours(1))
            .protocol()
            .cookie_store
    ));
}

#[tokio::test]
async fn pending_store_requires_browser_binding_and_expires() {
    let store = PendingLoginStore::new(2);
    let now = OffsetDateTime::now_utc();
    let pending = store
        .create(now, time::Duration::minutes(5))
        .expect("pending login should be created");
    let different = store
        .create(now, time::Duration::minutes(5))
        .expect("second pending login should be created");

    assert!(store.authorize(pending.id(), pending.browser_binding(), now));
    assert!(!store.authorize(pending.id(), different.browser_binding(), now));

    pending
        .transition(PendingLoginState::WaitingForQr)
        .await
        .expect("valid transition");
    pending
        .transition(PendingLoginState::WaitingForScan)
        .await
        .expect("valid transition");
    let mut events = pending.subscribe();
    pending.publish(LoginEvent::Scanned);
    assert_eq!(
        events.recv().await.expect("event should arrive"),
        LoginEvent::Scanned
    );

    assert_eq!(store.expire(now + time::Duration::minutes(6)), 2);
    assert!(pending.cancellation().is_cancelled());
    assert_eq!(pending.state().await, PendingLoginState::Expired);
    assert_eq!(store.len(), 0);
}

#[tokio::test]
async fn completed_pending_login_is_claimed_once_by_its_browser() {
    let store = PendingLoginStore::new(1);
    let now = OffsetDateTime::now_utc();
    let pending = store
        .create(now, time::Duration::minutes(5))
        .expect("pending login should be created");
    let other_binding = PendingLoginStore::new(1)
        .create(now, time::Duration::minutes(5))
        .expect("other pending should be created")
        .browser_binding()
        .clone();
    let login = AuthenticatedLogin {
        context: protocol_context(),
        identity: identity("user-a"),
    };

    pending
        .transition(PendingLoginState::WaitingForQr)
        .await
        .expect("valid transition");
    pending
        .transition(PendingLoginState::WaitingForScan)
        .await
        .expect("valid transition");
    pending
        .transition(PendingLoginState::Authenticating)
        .await
        .expect("valid transition");
    pending
        .complete(login)
        .await
        .expect("pending completion should succeed");
    assert!(
        store
            .claim_authenticated(pending.id(), &other_binding, now)
            .is_none()
    );
    assert!(
        store
            .claim_authenticated(pending.id(), pending.browser_binding(), now)
            .is_some()
    );
    assert!(
        store
            .claim_authenticated(pending.id(), pending.browser_binding(), now)
            .is_none()
    );
    assert_eq!(store.len(), 0);
}

#[tokio::test]
async fn unclaimed_completed_login_is_destroyed_after_short_grace() {
    let store = PendingLoginStore::new(1);
    let now = OffsetDateTime::now_utc();
    let pending = store
        .create(now, time::Duration::minutes(5))
        .expect("pending login");
    pending
        .transition(PendingLoginState::WaitingForQr)
        .await
        .expect("waiting QR");
    pending
        .transition(PendingLoginState::WaitingForScan)
        .await
        .expect("waiting scan");
    pending
        .transition(PendingLoginState::Authenticating)
        .await
        .expect("authenticating");
    pending
        .complete(AuthenticatedLogin {
            context: protocol_context(),
            identity: identity("user-a"),
        })
        .await
        .expect("complete");

    assert_eq!(store.expire(now + time::Duration::seconds(31)), 1);
    assert!(
        store
            .claim_authenticated(
                pending.id(),
                pending.browser_binding(),
                now + time::Duration::seconds(31)
            )
            .is_none()
    );
}
