use secrecy::SecretString;
use url::{Url, form_urlencoded};

use crate::error::ProtocolError;

pub fn extract_token_id(url: &Url) -> Result<SecretString, ProtocolError> {
    let mut values = url
        .query_pairs()
        .filter(|(key, _)| key == "tokenId")
        .map(|(_, value)| value.into_owned())
        .collect::<Vec<_>>();
    if let Some(fragment_query) = url.fragment().and_then(fragment_query) {
        values.extend(
            form_urlencoded::parse(fragment_query.as_bytes())
                .filter(|(key, _)| key == "tokenId")
                .map(|(_, value)| value.into_owned()),
        );
    }
    if values.len() != 1 || values[0].is_empty() {
        return Err(ProtocolError::TokenIdMissing);
    }
    Ok(SecretString::from(values.remove(0)))
}

fn fragment_query(fragment: &str) -> Option<&str> {
    fragment.split_once('?').map(|(_, query)| query)
}
