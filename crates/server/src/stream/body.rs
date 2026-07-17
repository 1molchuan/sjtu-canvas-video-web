use std::pin::Pin;

use axum::body::Body;
use bytes::Bytes;
use futures_util::{Stream, StreamExt, stream};
use tokio::sync::OwnedSemaphorePermit;
use tokio_util::sync::CancellationToken;

pub struct DownloadPermits {
    _global: OwnedSemaphorePermit,
    _user: OwnedSemaphorePermit,
}

impl DownloadPermits {
    pub fn new(global: OwnedSemaphorePermit, user: OwnedSemaphorePermit) -> Self {
        Self {
            _global: global,
            _user: user,
        }
    }
}

struct BodyState {
    upstream: Pin<Box<dyn Stream<Item = Result<Bytes, reqwest::Error>> + Send>>,
    _permits: DownloadPermits,
    session_revoked: CancellationToken,
    shutting_down: CancellationToken,
    request_id: String,
    bytes_streamed: u64,
    outcome: StreamOutcome,
}

pub struct StreamingBodyOptions {
    pub permits: DownloadPermits,
    pub session_revoked: CancellationToken,
    pub shutting_down: CancellationToken,
}

#[derive(Clone, Copy)]
enum StreamOutcome {
    Open,
    Complete,
    UpstreamError,
    SessionRevoked,
    ShuttingDown,
}

pub fn streaming_body(response: reqwest::Response, options: StreamingBodyOptions) -> Body {
    let state = BodyState {
        upstream: Box::pin(response.bytes_stream()),
        _permits: options.permits,
        session_revoked: options.session_revoked,
        shutting_down: options.shutting_down,
        request_id: crate::middleware::request_id::current(),
        bytes_streamed: 0,
        outcome: StreamOutcome::Open,
    };
    Body::from_stream(stream::unfold(state, next_chunk))
}

async fn next_chunk(mut state: BodyState) -> Option<(Result<Bytes, reqwest::Error>, BodyState)> {
    tokio::select! {
        biased;
        _ = state.session_revoked.cancelled() => finish(state, StreamOutcome::SessionRevoked),
        _ = state.shutting_down.cancelled() => finish(state, StreamOutcome::ShuttingDown),
        next = state.upstream.next() => map_chunk(state, next),
    }
}

fn finish(
    mut state: BodyState,
    outcome: StreamOutcome,
) -> Option<(Result<Bytes, reqwest::Error>, BodyState)> {
    state.outcome = outcome;
    None
}

fn map_chunk(
    mut state: BodyState,
    next: Option<Result<Bytes, reqwest::Error>>,
) -> Option<(Result<Bytes, reqwest::Error>, BodyState)> {
    let Some(item) = next else {
        if matches!(state.outcome, StreamOutcome::Open) {
            state.outcome = StreamOutcome::Complete;
        }
        return None;
    };
    match &item {
        Ok(bytes) => state.bytes_streamed += bytes.len() as u64,
        Err(_) => state.outcome = StreamOutcome::UpstreamError,
    }
    Some((item, state))
}

impl StreamOutcome {
    fn as_str(self) -> &'static str {
        match self {
            Self::Open => "client_disconnected",
            Self::Complete => "complete",
            Self::UpstreamError => "upstream_error",
            Self::SessionRevoked => "session_revoked",
            Self::ShuttingDown => "shutting_down",
        }
    }
}

impl Drop for BodyState {
    fn drop(&mut self) {
        tracing::info!(
            request_id = %self.request_id,
            route = "/api/download/{ticket}",
            bytes_streamed = self.bytes_streamed,
            outcome = self.outcome.as_str(),
            "download stream closed"
        );
    }
}
