use axum::{
    extract::State,
    response::sse::{Event, KeepAlive, Sse},
};
use futures::stream::Stream;
use std::{convert::Infallible, time::Duration};
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt;

use crate::core::app::AppState;

pub async fn realtime_sse(
    State(state): State<AppState>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let rx = state.realtime.subscribe();
    let stream = BroadcastStream::new(rx).filter_map(|msg| match msg {
        Ok(m) => {
            let json_str = serde_json::to_string(&m).unwrap_or_default();
            Some(Ok(Event::default().data(json_str)))
        }
        Err(_) => None,
    });

    Sse::new(stream).keep_alive(
        KeepAlive::new()
            .interval(Duration::from_secs(15))
            .text("keep-alive-ping"),
    )
}
