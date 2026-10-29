//! Response formatting & Debug Profiler for padiBase (ported from Padi Rust Api Framework)
//!
//! Formats JSON responses according to `RESPONSE_FORMAT` (`full` | `simple` | `raw`)
//! and attaches `debug` execution metrics & SQL query logs when `APP_DEBUG=true`.

use axum::{
    body::{to_bytes, Body},
    extract::{Request, State},
    http::{header, HeaderValue, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use serde_json::{json, Map, Value};

use super::app::AppState;
use super::db::{current_queries, current_query_count, REQUEST_QUERIES};
use super::env::Env;

tokio::task_local! {
    pub static REQUEST_START: std::time::Instant;
}

/// Get standardized UPPERCASE status/error code name (matching `padi_core::response::status_code_name`).
pub fn status_code_name(code: u16) -> &'static str {
    match code {
        201 => "CREATED",
        204 => "NO_CONTENT",
        400 => "BAD_REQUEST",
        401 => "UNAUTHORIZED",
        403 => "FORBIDDEN",
        404 => "NOT_FOUND",
        405 => "METHOD_NOT_ALLOWED",
        409 => "CONFLICT",
        422 => "VALIDATION_FAILED",
        429 => "TOO_MANY_REQUESTS",
        500 => "INTERNAL_SERVER_ERROR",
        502 => "BAD_GATEWAY",
        503 => "SERVICE_UNAVAILABLE",
        _ => "SUCCESS",
    }
}

/// Transform a JSON value into Padi Rust Api Framework response format (`full` | `simple` | `raw`).
pub fn format_padi_json(
    status: u16,
    raw: Value,
    db_driver: &str,
    elapsed_ms: f64,
) -> Value {
    let format = Env::get_or("RESPONSE_FORMAT", "full").to_lowercase();
    if format == "raw" {
        return raw;
    }

    let is_success = status < 400;
    let code_name = status_code_name(status);
    let mut out = Map::new();

    if !is_success {
        let msg = raw
            .get("message")
            .and_then(|v| v.as_str())
            .unwrap_or("Error")
            .to_string();

        if format == "simple" {
            out.insert("status".to_string(), json!("error"));
            out.insert("code".to_string(), json!(code_name));
            out.insert("message".to_string(), json!(msg));
        } else {
            out.insert("message".to_string(), json!(msg));
            out.insert("message_code".to_string(), json!(code_name));
            out.insert("success".to_string(), json!(false));
            out.insert("code".to_string(), json!(status));
        }
    } else if let Value::Object(ref obj) = raw {
        let is_paginated_list = obj.contains_key("items") && obj.contains_key("totalItems");
        let is_simple_items_list = obj.contains_key("items") && !obj.contains_key("page");

        if format == "simple" {
            out.insert("status".to_string(), json!("success"));
            out.insert("code".to_string(), json!(code_name));
            if is_paginated_list {
                let items = obj.get("items").cloned().unwrap_or_else(|| json!([]));
                let page = obj.get("page").and_then(|v| v.as_i64()).unwrap_or(1);
                let per_page = obj.get("perPage").and_then(|v| v.as_i64()).unwrap_or(30);
                let total = obj.get("totalItems").and_then(|v| v.as_i64()).unwrap_or(0);
                let last_page = obj.get("totalPages").and_then(|v| v.as_i64()).unwrap_or(1);
                let len = items.as_array().map(|a| a.len() as i64).unwrap_or(0);
                let offset = (page - 1) * per_page;
                let from = if total > 0 { offset + 1 } else { 0 };
                let to = (offset + len).min(total);

                out.insert("item".to_string(), items);
                out.insert(
                    "meta".to_string(),
                    json!({
                        "current_page": page,
                        "per_page": per_page,
                        "total": total,
                        "last_page": last_page,
                        "from": from,
                        "to": to
                    }),
                );
            } else if is_simple_items_list {
                out.insert(
                    "item".to_string(),
                    obj.get("items").cloned().unwrap_or_else(|| json!([])),
                );
            } else {
                out.insert("item".to_string(), raw.clone());
            }
        } else {
            // RESPONSE_FORMAT = full (Padi Rust default)
            let msg = obj
                .get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("Success")
                .to_string();

            out.insert("message".to_string(), json!(msg));
            out.insert("message_code".to_string(), json!(code_name));
            out.insert("success".to_string(), json!(true));

            if is_paginated_list {
                let items = obj.get("items").cloned().unwrap_or_else(|| json!([]));
                let page = obj.get("page").and_then(|v| v.as_i64()).unwrap_or(1);
                let per_page = obj.get("perPage").and_then(|v| v.as_i64()).unwrap_or(30);
                let total = obj.get("totalItems").and_then(|v| v.as_i64()).unwrap_or(0);
                let last_page = obj.get("totalPages").and_then(|v| v.as_i64()).unwrap_or(1);
                let len = items.as_array().map(|a| a.len() as i64).unwrap_or(0);
                let offset = (page - 1) * per_page;
                let from = if total > 0 { offset + 1 } else { 0 };
                let to = (offset + len).min(total);

                out.insert("item".to_string(), items);
                out.insert(
                    "meta".to_string(),
                    json!({
                        "current_page": page,
                        "per_page": per_page,
                        "total": total,
                        "last_page": last_page,
                        "from": from,
                        "to": to
                    }),
                );
            } else if is_simple_items_list {
                out.insert(
                    "item".to_string(),
                    obj.get("items").cloned().unwrap_or_else(|| json!([])),
                );
            } else {
                out.insert("item".to_string(), raw.clone());
            }
        }
    } else {
        out.insert("message".to_string(), json!("Success"));
        out.insert("message_code".to_string(), json!(code_name));
        out.insert("success".to_string(), json!(true));
        out.insert("item".to_string(), raw);
    }

    // Attach `debug` block in development / debug mode (matching Padi Rust Api Framework)
    if Env::is_debug() || Env::is_development() {
        out.insert(
            "debug".to_string(),
            json!({
                "env": Env::get_or("APP_ENV", "development"),
                "database": db_driver,
                "execution_time": format!("{:.2}ms", elapsed_ms),
                "queries_count": current_query_count(),
                "queries": current_queries(),
                "timestamp": chrono::Local::now().to_rfc3339(),
            }),
        );
    }

    Value::Object(out)
}

/// Axum middleware that profiles request execution time & SQL queries and wraps JSON responses in Padi Rust format.
pub async fn padi_response_middleware(
    State(state): State<AppState>,
    req: Request,
    next: Next,
) -> Response {
    let is_api_route = req.uri().path().starts_with("/api");
    let start = std::time::Instant::now();
    let queries = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));

    REQUEST_QUERIES
        .scope(queries, async move {
            let res = REQUEST_START
                .scope(start, async move { next.run(req).await })
                .await;

            if !is_api_route {
                return res;
            }

            let status = res.status();
            if status == StatusCode::NO_CONTENT {
                return res;
            }

            // Skip SSE streams or file attachments
            let is_attachment = res
                .headers()
                .get(header::CONTENT_DISPOSITION)
                .and_then(|v| v.to_str().ok())
                .map(|s| s.contains("attachment"))
                .unwrap_or(false);
            if is_attachment {
                return res;
            }

            let is_json = res
                .headers()
                .get(header::CONTENT_TYPE)
                .and_then(|v| v.to_str().ok())
                .map(|s| s.contains("application/json"))
                .unwrap_or(false);
            if !is_json {
                return res;
            }

            let elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;
            let db_driver = state.db.driver_name();

            let (mut parts, body) = res.into_parts();
            let Ok(bytes) = to_bytes(body, 64 * 1024 * 1024).await else {
                return StatusCode::INTERNAL_SERVER_ERROR.into_response();
            };

            let Ok(raw_json) = serde_json::from_slice::<Value>(&bytes) else {
                return Response::from_parts(parts, Body::from(bytes));
            };

            let formatted = format_padi_json(status.as_u16(), raw_json, db_driver, elapsed_ms);
            let out_bytes = if Env::is_development() || Env::is_debug() {
                serde_json::to_vec_pretty(&formatted).unwrap_or_else(|_| bytes.to_vec())
            } else {
                serde_json::to_vec(&formatted).unwrap_or_else(|_| bytes.to_vec())
            };

            parts.headers.remove(header::CONTENT_LENGTH);
            parts.headers.insert(
                header::CONTENT_TYPE,
                HeaderValue::from_static("application/json; charset=utf-8"),
            );

            Response::from_parts(parts, Body::from(out_bytes))
        })
        .await
}
