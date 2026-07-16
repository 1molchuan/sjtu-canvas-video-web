use futures_util::StreamExt;

use crate::error::ProtocolError;

pub async fn read_limited_body(
    response: reqwest::Response,
    max_bytes: usize,
) -> Result<Vec<u8>, ProtocolError> {
    if response
        .content_length()
        .is_some_and(|length| length > max_bytes as u64)
    {
        return Err(ProtocolError::UpstreamResponseTooLarge);
    }
    let mut stream = response.bytes_stream();
    let mut body = Vec::new();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|_| ProtocolError::UpstreamBodyReadFailed)?;
        if body.len().saturating_add(chunk.len()) > max_bytes {
            return Err(ProtocolError::UpstreamResponseTooLarge);
        }
        body.extend_from_slice(&chunk);
    }
    Ok(body)
}
