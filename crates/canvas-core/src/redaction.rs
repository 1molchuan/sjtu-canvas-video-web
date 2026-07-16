use std::fmt;

use sha2::{Digest, Sha256};
use url::Url;

const HASH_BYTES_TO_DISPLAY: usize = 8;

#[derive(Clone)]
pub struct Redactor {
    key: [u8; 32],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SanitizedUrl {
    scheme: String,
    host: String,
    port: Option<u16>,
    path_hash: String,
    query_present: bool,
}

impl Redactor {
    pub const fn new(key: [u8; 32]) -> Self {
        Self { key }
    }

    pub fn hash_identifier(&self, value: &str) -> String {
        self.hash_bytes(value.as_bytes())
    }

    pub fn sanitize_url(&self, url: &Url) -> SanitizedUrl {
        SanitizedUrl {
            scheme: url.scheme().to_owned(),
            host: url.host_str().unwrap_or("<missing>").to_owned(),
            port: url.port(),
            path_hash: self.hash_bytes(url.path().as_bytes()),
            query_present: url.query().is_some(),
        }
    }

    fn hash_bytes(&self, value: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(self.key);
        hasher.update(value);
        let digest = hasher.finalize();
        digest[..HASH_BYTES_TO_DISPLAY]
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect()
    }
}

impl fmt::Display for SanitizedUrl {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "scheme={} host={} port={:?} path_hash={} query_present={}",
            self.scheme, self.host, self.port, self.path_hash, self.query_present
        )
    }
}

impl SanitizedUrl {
    pub fn host(&self) -> &str {
        &self.host
    }

    pub fn query_present(&self) -> bool {
        self.query_present
    }
}
