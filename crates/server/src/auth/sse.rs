use std::{sync::Arc, time::Duration};

use axum::response::sse::{Event, KeepAlive, Sse};
use futures_util::{Stream, StreamExt, stream};
use tokio::sync::broadcast;

use super::pending::{LoginEvent, PendingLogin};

struct LiveState {
    receiver: broadcast::Receiver<LoginEvent>,
    seen: Vec<LoginEvent>,
    done: bool,
}

pub fn response(
    pending: &Arc<PendingLogin>,
) -> Sse<impl Stream<Item = Result<Event, axum::Error>> + Send + 'static> {
    let receiver = pending.subscribe();
    let history = pending.event_history();
    let done = history.last().is_some_and(LoginEvent::is_terminal);
    let live_state = LiveState {
        receiver,
        seen: history.clone(),
        done,
    };
    let history_stream = stream::iter(history.into_iter().map(to_event));
    let live_stream = stream::unfold(live_state, next_live).map(to_event);
    Sse::new(history_stream.chain(live_stream)).keep_alive(
        KeepAlive::new()
            .interval(Duration::from_secs(15))
            .text("keep-alive"),
    )
}

async fn next_live(mut state: LiveState) -> Option<(LoginEvent, LiveState)> {
    if state.done {
        return None;
    }
    loop {
        match state.receiver.recv().await {
            Ok(event) if state.seen.contains(&event) => continue,
            Ok(event) => {
                state.done = event.is_terminal();
                state.seen.push(event.clone());
                return Some((event, state));
            }
            Err(broadcast::error::RecvError::Lagged(_)) => {
                state.done = true;
                return Some((lagged_event(), state));
            }
            Err(broadcast::error::RecvError::Closed) => return None,
        }
    }
}

fn to_event(event: LoginEvent) -> Result<Event, axum::Error> {
    Event::default().json_data(event)
}

fn lagged_event() -> LoginEvent {
    LoginEvent::Error {
        code: "SSE_LAGGED".to_owned(),
        message: "登录状态更新过快，请重新连接。".to_owned(),
    }
}
