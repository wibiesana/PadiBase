use axum::{
    extract::{Query, State},
    http::{HeaderMap, StatusCode},
    response::sse::{Event, KeepAlive, Sse},
    response::{IntoResponse, Response},
    Json,
};
use serde::Deserialize;
use serde_json::json;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::{convert::Infallible, time::Duration};
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt;

use crate::core::app::AppState;
use crate::core::auth::{authenticate_with_query_token, check_permission, Claims, RecordAction};
use crate::core::db::get_collection_by_name_or_id;
use crate::core::env::Env;

#[derive(Deserialize, Default)]
pub struct RealtimeQuery {
    /// `EventSource` cannot set headers, so a token may be passed as `?token=`.
    pub token: Option<String>,
}

static ACTIVE_STREAMS: AtomicUsize = AtomicUsize::new(0);

/// Releases one SSE slot when the client disconnects and the stream is dropped.
struct StreamSlot;

impl Drop for StreamSlot {
    fn drop(&mut self) {
        ACTIVE_STREAMS.fetch_sub(1, Ordering::AcqRel);
    }
}

fn max_streams() -> usize {
    Env::get_int("REALTIME_MAX_CONNECTIONS", 1000).max(1) as usize
}

/// Realtime events are filtered per subscriber with the collection's `list` rule, so a client
/// never receives events for collections it could not list through the REST API.
pub async fn realtime_sse(
    State(state): State<AppState>,
    Query(query): Query<RealtimeQuery>,
    headers: HeaderMap,
) -> Response {
    let auth = match authenticate_with_query_token(&state, &headers, query.token.as_deref()).await {
        Ok(a) => a,
        Err(e) => return e.into_response(),
    };

    // Connection cap (each stream holds a broadcast receiver + task)
    if ACTIVE_STREAMS.fetch_add(1, Ordering::AcqRel) >= max_streams() {
        ACTIVE_STREAMS.fetch_sub(1, Ordering::AcqRel);
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({ "code": 503, "message": "Too many realtime connections." })),
        )
            .into_response();
    }
    let slot = Arc::new(StreamSlot);

    let rx = state.realtime.subscribe();
    let auth: Option<Arc<Claims>> = auth.map(Arc::new);
    let stream = BroadcastStream::new(rx).then(move |msg| {
        let state = state.clone();
        let auth = auth.clone();
        let _slot = slot.clone();
        async move {
            match msg {
                Ok(m) => {
                    let allowed = match get_collection_by_name_or_id(&state.db, &m.table).await {
                        Ok(Some(col)) => {
                            check_permission(RecordAction::List, &col, auth.as_deref(), None).is_ok()
                        }
                        _ => false,
                    };
                    if !allowed {
                        return None;
                    }
                    let json_str = serde_json::to_string(&m).unwrap_or_default();
                    Some(Ok::<Event, Infallible>(Event::default().data(json_str)))
                }
                // Slow consumer: tell the client it missed events so it can resync.
                Err(_) => Some(Ok(Event::default().event("lagged").data("resync"))),
            }
        }
    });
    let stream = stream.filter_map(|e| e);

    Sse::new(stream)
        .keep_alive(
            KeepAlive::new()
                .interval(Duration::from_secs(15))
                .text("keep-alive-ping"),
        )
        .into_response()
}
