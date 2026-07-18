use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};
use thiserror::Error;
use time::{Duration, OffsetDateTime};

use super::model::{
    AllowedIdentity, InviteCreation, InviteReservation, hash_token, invite_id, random_token,
    validate_token,
};

#[derive(Clone)]
pub struct InviteStore {
    path: Arc<PathBuf>,
}

#[derive(Debug, Error)]
pub enum InviteError {
    #[error("invite store operation failed")]
    Database(#[from] rusqlite::Error),
    #[error("invite store file operation failed")]
    Io(#[from] std::io::Error),
    #[error("secure random generation failed")]
    Random,
    #[error("invitation token is invalid")]
    Invalid,
    #[error("invitation has expired")]
    Expired,
    #[error("invitation is already reserved")]
    Reserved,
    #[error("invitation has already been consumed")]
    Consumed,
}

impl InviteStore {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, InviteError> {
        let store = Self {
            path: Arc::new(path.as_ref().to_owned()),
        };
        initialize(&store.connection()?)?;
        restrict_permissions(path.as_ref())?;
        Ok(store)
    }

    pub fn create(
        &self,
        now: OffsetDateTime,
        ttl: Duration,
    ) -> Result<InviteCreation, InviteError> {
        let token = random_token()?;
        let token_hash = hash_token(secrecy::ExposeSecret::expose_secret(&token));
        let id = invite_id(&token_hash);
        let expires_at = now + ttl;
        self.connection()?.execute(
            "INSERT INTO invites(token_hash, created_at, expires_at) VALUES (?1, ?2, ?3)",
            params![
                token_hash,
                now.unix_timestamp(),
                expires_at.unix_timestamp()
            ],
        )?;
        Ok(InviteCreation {
            token,
            id,
            expires_at,
        })
    }

    pub fn reserve(
        &self,
        token: &secrecy::SecretString,
        now: OffsetDateTime,
        ttl: Duration,
    ) -> Result<InviteReservation, InviteError> {
        let exposed = secrecy::ExposeSecret::expose_secret(token);
        validate_token(exposed)?;
        let token_hash = hash_token(exposed);
        let random = random_token()?;
        let reservation_hash = hash_token(secrecy::ExposeSecret::expose_secret(&random));
        let mut connection = self.connection()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let row = invitation_row(&transaction, &token_hash)?.ok_or(InviteError::Invalid)?;
        validate_reservation_state(&row, now)?;
        transaction.execute(
            "UPDATE invites SET reservation_hash = ?1, reservation_expires_at = ?2 \
             WHERE token_hash = ?3",
            params![reservation_hash, (now + ttl).unix_timestamp(), token_hash],
        )?;
        transaction.commit()?;
        Ok(InviteReservation {
            token_hash,
            reservation_hash,
        })
    }

    pub fn release(&self, reservation: &InviteReservation) -> Result<(), InviteError> {
        self.connection()?.execute(
            "UPDATE invites SET reservation_hash = NULL, reservation_expires_at = NULL \
             WHERE token_hash = ?1 AND reservation_hash = ?2 AND consumed_at IS NULL",
            params![reservation.token_hash, reservation.reservation_hash],
        )?;
        Ok(())
    }

    pub fn consume_and_enroll(
        &self,
        reservation: &InviteReservation,
        stable_id_hash: &str,
        now: OffsetDateTime,
    ) -> Result<(), InviteError> {
        let mut connection = self.connection()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let row =
            invitation_row(&transaction, &reservation.token_hash)?.ok_or(InviteError::Invalid)?;
        validate_consumption(&row, reservation, now)?;
        transaction.execute(
            "INSERT OR IGNORE INTO allowed_identities(stable_id_hash, invite_id, enrolled_at) \
             VALUES (?1, ?2, ?3)",
            params![
                stable_id_hash,
                invite_id(&reservation.token_hash),
                now.unix_timestamp()
            ],
        )?;
        transaction.execute(
            "UPDATE invites SET consumed_at = ?1, reservation_hash = NULL, \
             reservation_expires_at = NULL WHERE token_hash = ?2",
            params![now.unix_timestamp(), reservation.token_hash],
        )?;
        transaction.commit()?;
        Ok(())
    }

    pub fn is_allowed(&self, stable_id_hash: &str) -> Result<bool, InviteError> {
        let found = self
            .connection()?
            .query_row(
                "SELECT 1 FROM allowed_identities WHERE stable_id_hash = ?1",
                [stable_id_hash],
                |_| Ok(()),
            )
            .optional()?;
        Ok(found.is_some())
    }

    pub fn list_allowed(&self) -> Result<Vec<AllowedIdentity>, InviteError> {
        let connection = self.connection()?;
        let mut statement = connection.prepare(
            "SELECT invite_id, enrolled_at FROM allowed_identities ORDER BY enrolled_at",
        )?;
        let rows = statement.query_map([], |row| {
            let timestamp: i64 = row.get(1)?;
            let enrolled_at = OffsetDateTime::from_unix_timestamp(timestamp).map_err(|error| {
                rusqlite::Error::FromSqlConversionFailure(
                    1,
                    rusqlite::types::Type::Integer,
                    Box::new(error),
                )
            })?;
            Ok(AllowedIdentity::new(row.get(0)?, enrolled_at))
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    pub fn revoke(&self, invite_id: &str) -> Result<bool, InviteError> {
        let changed = self.connection()?.execute(
            "DELETE FROM allowed_identities WHERE invite_id = ?1",
            [invite_id],
        )?;
        Ok(changed > 0)
    }

    fn connection(&self) -> Result<Connection, rusqlite::Error> {
        Connection::open(self.path.as_ref())
    }
}

struct InvitationRow {
    expires_at: i64,
    reservation_hash: Option<String>,
    reservation_expires_at: Option<i64>,
    consumed_at: Option<i64>,
}

fn initialize(connection: &Connection) -> Result<(), rusqlite::Error> {
    connection.execute_batch(
        "PRAGMA journal_mode=DELETE; PRAGMA synchronous=FULL; PRAGMA busy_timeout=5000; \
         CREATE TABLE IF NOT EXISTS invites(\
           token_hash TEXT PRIMARY KEY, created_at INTEGER NOT NULL, expires_at INTEGER NOT NULL,\
           reservation_hash TEXT, reservation_expires_at INTEGER, consumed_at INTEGER); \
         CREATE TABLE IF NOT EXISTS allowed_identities(\
           stable_id_hash TEXT PRIMARY KEY, invite_id TEXT NOT NULL UNIQUE,\
           enrolled_at INTEGER NOT NULL);",
    )
}

fn invitation_row(
    connection: &Connection,
    token_hash: &str,
) -> Result<Option<InvitationRow>, rusqlite::Error> {
    connection
        .query_row(
            "SELECT expires_at, reservation_hash, reservation_expires_at, consumed_at \
             FROM invites WHERE token_hash = ?1",
            [token_hash],
            |row| {
                Ok(InvitationRow {
                    expires_at: row.get(0)?,
                    reservation_hash: row.get(1)?,
                    reservation_expires_at: row.get(2)?,
                    consumed_at: row.get(3)?,
                })
            },
        )
        .optional()
}

fn validate_reservation_state(row: &InvitationRow, now: OffsetDateTime) -> Result<(), InviteError> {
    if row.consumed_at.is_some() {
        return Err(InviteError::Consumed);
    }
    if row.expires_at <= now.unix_timestamp() {
        return Err(InviteError::Expired);
    }
    let active = row.reservation_hash.is_some()
        && row
            .reservation_expires_at
            .is_some_and(|expiry| expiry > now.unix_timestamp());
    if active {
        return Err(InviteError::Reserved);
    }
    Ok(())
}

fn validate_consumption(
    row: &InvitationRow,
    reservation: &InviteReservation,
    now: OffsetDateTime,
) -> Result<(), InviteError> {
    if row.consumed_at.is_some() {
        return Err(InviteError::Consumed);
    }
    if row.expires_at <= now.unix_timestamp() {
        return Err(InviteError::Expired);
    }
    let matches = row.reservation_hash.as_deref() == Some(&reservation.reservation_hash)
        && row
            .reservation_expires_at
            .is_some_and(|expiry| expiry > now.unix_timestamp());
    if !matches {
        return Err(InviteError::Invalid);
    }
    Ok(())
}

#[cfg(unix)]
fn restrict_permissions(path: &Path) -> Result<(), InviteError> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;
    Ok(())
}

#[cfg(not(unix))]
fn restrict_permissions(_path: &Path) -> Result<(), InviteError> {
    Ok(())
}
