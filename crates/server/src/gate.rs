use std::ffi::OsStr;

use thiserror::Error;

pub const REAL_PROTOCOL_ENV: &str = "SJTU_REAL_PROTOCOL_TEST";

#[derive(Debug, Error)]
#[error("real protocol access is disabled; explicitly set SJTU_REAL_PROTOCOL_TEST=1")]
pub struct RealProtocolDisabled;

pub fn ensure_real_protocol_enabled() -> Result<(), RealProtocolDisabled> {
    if is_real_protocol_enabled(std::env::var_os(REAL_PROTOCOL_ENV).as_deref()) {
        return Ok(());
    }
    Err(RealProtocolDisabled)
}

pub fn is_real_protocol_enabled(value: Option<&OsStr>) -> bool {
    value == Some(OsStr::new("1"))
}

#[cfg(test)]
mod tests {
    use std::ffi::OsStr;

    use super::is_real_protocol_enabled;

    #[test]
    fn only_exact_one_enables_real_protocol() {
        assert!(is_real_protocol_enabled(Some(OsStr::new("1"))));
        assert!(!is_real_protocol_enabled(Some(OsStr::new("true"))));
        assert!(!is_real_protocol_enabled(Some(OsStr::new(" 1 "))));
        assert!(!is_real_protocol_enabled(None));
    }
}
