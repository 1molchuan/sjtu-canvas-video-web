use secrecy::SecretString;
use time::OffsetDateTime;

use crate::{
    error::WebError,
    invite::{InviteError, InviteReservation},
    state::AppState,
};

pub(crate) async fn reserve(
    state: &AppState,
    token: Option<String>,
    now: OffsetDateTime,
    ttl: time::Duration,
) -> Result<Option<InviteReservation>, WebError> {
    let Some(token) = token else {
        return Ok(None);
    };
    let store = state
        .invites()
        .cloned()
        .ok_or_else(WebError::invite_invalid)?;
    let token = SecretString::from(token);
    tokio::task::spawn_blocking(move || store.reserve(&token, now, ttl))
        .await
        .map_err(|_| WebError::internal())?
        .map(Some)
        .map_err(web_error)
}

pub(crate) async fn release(
    state: &AppState,
    reservation: Option<InviteReservation>,
) -> Result<(), WebError> {
    let Some(reservation) = reservation else {
        return Ok(());
    };
    let store = state.invites().cloned().ok_or_else(WebError::internal)?;
    tokio::task::spawn_blocking(move || store.release(&reservation))
        .await
        .map_err(|_| WebError::internal())?
        .map_err(|_| WebError::internal())
}

fn web_error(error: InviteError) -> WebError {
    match error {
        InviteError::Invalid => WebError::invite_invalid(),
        InviteError::Expired => WebError::invite_expired(),
        InviteError::Reserved | InviteError::Consumed => WebError::invite_already_used(),
        InviteError::Database(_) | InviteError::Io(_) | InviteError::Random => WebError::internal(),
    }
}
