use secrecy::ExposeSecret;
use subtle::ConstantTimeEq;

use crate::session::UserSession;

pub fn token(session: &UserSession) -> String {
    session.csrf_secret().expose_secret().to_owned()
}

pub fn verify(session: &UserSession, candidate: &str) -> bool {
    let expected = session.csrf_secret().expose_secret().as_bytes();
    let candidate = candidate.as_bytes();
    expected.len() == candidate.len() && bool::from(expected.ct_eq(candidate))
}
