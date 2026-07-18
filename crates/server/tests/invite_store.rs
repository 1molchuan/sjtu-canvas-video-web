use std::{fs, path::PathBuf};

use secrecy::ExposeSecret;
use server::invite::{InviteError, InviteStore, invitation_url};
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

struct TestDatabase {
    path: PathBuf,
}

impl TestDatabase {
    fn new() -> Self {
        Self {
            path: std::env::temp_dir()
                .join(format!("canvas-video-invites-{}.sqlite3", Uuid::new_v4())),
        }
    }
}

impl Drop for TestDatabase {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
        let _ = fs::remove_file(self.path.with_extension("sqlite3-shm"));
        let _ = fs::remove_file(self.path.with_extension("sqlite3-wal"));
    }
}

#[test]
fn invitation_is_single_use_and_enrolls_identity_persistently() {
    let database = TestDatabase::new();
    let now = OffsetDateTime::from_unix_timestamp(1_800_000_000).expect("test time");
    let store = InviteStore::open(&database.path).expect("invite store");
    let invitation = store
        .create(now, Duration::hours(24))
        .expect("create invitation");

    let reservation = store
        .reserve(invitation.token(), now, Duration::minutes(5))
        .expect("reserve invitation");
    assert!(matches!(
        store.reserve(invitation.token(), now, Duration::minutes(5)),
        Err(InviteError::Reserved)
    ));

    store
        .consume_and_enroll(&reservation, "sha256:identity", now)
        .expect("consume invitation");
    assert!(store.is_allowed("sha256:identity").expect("lookup"));
    assert!(matches!(
        store.reserve(invitation.token(), now, Duration::minutes(5)),
        Err(InviteError::Consumed)
    ));

    drop(store);
    let reopened = InviteStore::open(&database.path).expect("reopen invite store");
    assert!(reopened.is_allowed("sha256:identity").expect("lookup"));
}

#[test]
fn expired_and_released_invitations_have_explicit_behavior() {
    let database = TestDatabase::new();
    let now = OffsetDateTime::from_unix_timestamp(1_800_000_000).expect("test time");
    let store = InviteStore::open(&database.path).expect("invite store");
    let invitation = store
        .create(now, Duration::hours(1))
        .expect("create invitation");
    let reservation = store
        .reserve(invitation.token(), now, Duration::minutes(5))
        .expect("reserve invitation");

    store.release(&reservation).expect("release reservation");
    store
        .reserve(invitation.token(), now, Duration::minutes(5))
        .expect("released invitation can be retried");
    assert!(matches!(
        store.reserve(
            invitation.token(),
            now + Duration::hours(2),
            Duration::minutes(5)
        ),
        Err(InviteError::Expired)
    ));
}

#[test]
fn raw_invite_token_is_not_persisted() {
    let database = TestDatabase::new();
    let now = OffsetDateTime::from_unix_timestamp(1_800_000_000).expect("test time");
    let store = InviteStore::open(&database.path).expect("invite store");
    let invitation = store
        .create(now, Duration::hours(24))
        .expect("create invitation");
    drop(store);

    let database_bytes = fs::read(&database.path).expect("read database");
    let token = invitation.token().expose_secret().as_bytes();
    assert!(
        !database_bytes
            .windows(token.len())
            .any(|window| window == token)
    );
}

#[test]
fn invitation_url_uses_browser_fragment_and_enrollment_can_be_revoked_by_invite_id() {
    let database = TestDatabase::new();
    let now = OffsetDateTime::from_unix_timestamp(1_800_000_000).expect("test time");
    let store = InviteStore::open(&database.path).expect("invite store");
    let invitation = store
        .create(now, Duration::hours(24))
        .expect("create invitation");
    let url = invitation_url("https://canvas-video.example.test", invitation.token())
        .expect("invitation URL");
    assert_eq!(url.path(), "/login");
    assert!(url.query().is_none());
    assert!(
        url.fragment()
            .is_some_and(|value| value.starts_with("invite="))
    );

    let reservation = store
        .reserve(invitation.token(), now, Duration::minutes(5))
        .expect("reserve");
    store
        .consume_and_enroll(&reservation, "sha256:revocable", now)
        .expect("enroll");
    let enrolled = store.list_allowed().expect("list allowed identities");
    assert_eq!(enrolled.len(), 1);
    assert_eq!(enrolled[0].invite_id(), invitation.id());
    assert!(store.revoke(invitation.id()).expect("revoke"));
    assert!(!store.is_allowed("sha256:revocable").expect("lookup"));
}
