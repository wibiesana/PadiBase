//! Response formatting & Debug Profiler for padiBase (ported from Padi Rust Api Framework)
//!
//! Formats JSON responses according to `RESPONSE_FORMAT` (`full` | `simple` | `raw`)
//! using lock-free cached config and zero-clone JSON envelope wrapping.

use axum::{
    body::{to_bytes, Body},
    extract::{Request, State},
    http::{header, HeaderMap, HeaderName, HeaderValue, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use serde_json::{json, Map, Value};
use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};
use std::time::{Duration, Instant};

use super::app::AppState;
use super::db::REQUEST_QUERIES;
use super::env::{Env, ResponseFormat};

tokio::task_local! {
    pub static REQUEST_START: std::time::Instant;
}

struct RateBucket {
    count: u64,
    window_start: Instant,
}

static RATE_LIMITER: LazyLock<Mutex<HashMap<String, RateBucket>>> =
    LazyLock::new(|| Mutex::new(HashMap::with_capacity(256)));

fn extract_client_ip(headers: &HeaderMap) -> String {
    if let Some(xff) = headers.get("x-forwarded-for").and_then(|v| v.to_str().ok()) {
        if let Some(first) = xff.split(',').next() {
            let trimmed = first.trim();
            if !trimmed.is_empty() {
                return trimmed.to_string();
            }
        }
    }
    if let Some(xri) = headers.get("x-real-ip").and_then(|v| v.to_str().ok()) {
        let trimmed = xri.trim();
        if !trimmed.is_empty() {
            return trimmed.to_string();
        }
    }
    "127.0.0.1".to_string()
}

/// Check rate limit for IP if `RATE_LIMIT_PER_MINUTE > 0`. Returns `Ok((limit, remaining))` or `Err(retry_after_secs)`.
fn check_rate_limit(ip: &str, max_per_min: u64) -> Result<(u64, u64), u64> {
    if max_per_min == 0 {
        return Ok((0, 0));
    }
    let now = Instant::now();
    let window = Duration::from_secs(60);
    let Ok(mut map) = RATE_LIMITER.lock() else {
        return Ok((max_per_min, max_per_min));
    };

    // Periodic cleanup if map grows large
    if map.len() > 4096 {
        map.retain(|_, b| now.duration_since(b.window_start) < window);
    }

    let bucket = map.entry(ip.to_string()).or_insert(RateBucket {
        count: 0,
        window_start: now,
    });

    let elapsed = now.duration_since(bucket.window_start);
    if elapsed >= window {
        bucket.count = 1;
        bucket.window_start = now;
        Ok((max_per_min, max_per_min.saturating_sub(1)))
    } else if bucket.count >= max_per_min {
        let retry_after = (60u64).saturating_sub(elapsed.as_secs()).max(1);
        Err(retry_after)
    } else {
        bucket.count += 1;
        Ok((max_per_min, max_per_min.saturating_sub(bucket.count)))
    }
}

#[inline]
fn apply_security_headers(headers: &mut HeaderMap, rate_info: Option<(u64, u64)>) {
    if Env::security_headers_enabled() {
        headers.insert(
            header::X_CONTENT_TYPE_OPTIONS,
            HeaderValue::from_static("nosniff"),
        );
        headers.insert(
            header::X_FRAME_OPTIONS,
            HeaderValue::from_static("SAMEORIGIN"),
        );
        headers.insert(
            HeaderName::from_static("x-xss-protection"),
            HeaderValue::from_static("1; mode=block"),
        );
        headers.insert(
            header::REFERRER_POLICY,
            HeaderValue::from_static("strict-origin-when-cross-origin"),
        );
        headers.insert(
            HeaderName::from_static("x-powered-by"),
            HeaderValue::from_static("PadiBase"),
        );
    }
    if let Some((limit, remaining)) = rate_info {
        if limit > 0 {
            if let Ok(v) = HeaderValue::from_str(&limit.to_string()) {
                headers.insert(HeaderName::from_static("x-ratelimit-limit"), v);
            }
            if let Ok(v) = HeaderValue::from_str(&remaining.to_string()) {
                headers.insert(HeaderName::from_static("x-ratelimit-remaining"), v);
            }
        }
    }
}

/// Get standardized UPPERCASE status/error code name (matching `padi_core::response::status_code_name`).
#[inline]
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

/// Transform a JSON value into Padi Rust Api Framework response format (`full` | `simple` | `raw`)
/// using O(1) in-place moves instead of deep cloning.
pub fn format_padi_json(
    status: u16,
    raw: Value,
    db_driver: &str,
    elapsed_ms: f64,
    queries: &[super::db::QueryLog],
) -> Value {
    let format = Env::response_format();
    if format == ResponseFormat::Raw {
        return raw;
    }

    let is_success = status < 400;
    let code_name = status_code_name(status);
    let mut out = Map::with_capacity(8);

    if !is_success {
        let msg = raw
            .get("message")
            .and_then(|v| v.as_str())
            .unwrap_or("Error")
            .to_string();

        if format == ResponseFormat::Simple {
            out.insert("status".to_string(), json!("error"));
            out.insert("code".to_string(), json!(code_name));
            out.insert("message".to_string(), Value::String(msg));
        } else {
            out.insert("message".to_string(), Value::String(msg));
            out.insert("message_code".to_string(), json!(code_name));
            out.insert("success".to_string(), Value::Bool(false));
            out.insert("code".to_string(), json!(status));
        }
    } else if let Value::Object(mut obj) = raw {
        let is_paginated_list = obj.contains_key("items") && obj.contains_key("totalItems");
        let is_simple_items_list = obj.contains_key("items") && !obj.contains_key("page");

        if format == ResponseFormat::Simple {
            out.insert("status".to_string(), json!("success"));
            out.insert("code".to_string(), json!(code_name));
            if is_paginated_list {
                let page = obj.get("page").and_then(|v| v.as_i64()).unwrap_or(1);
                let per_page = obj.get("perPage").and_then(|v| v.as_i64()).unwrap_or(30);
                let total = obj.get("totalItems").and_then(|v| v.as_i64()).unwrap_or(0);
                let last_page = obj.get("totalPages").and_then(|v| v.as_i64()).unwrap_or(1);
                let items = obj.remove("items").unwrap_or_else(|| json!([]));
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
                let items = obj.remove("items").unwrap_or_else(|| json!([]));
                out.insert("item".to_string(), items);
            } else {
                out.insert("item".to_string(), Value::Object(obj));
            }
        } else {
            // RESPONSE_FORMAT = full (Padi Rust default)
            let msg = obj
                .get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("Success")
                .to_string();

            out.insert("message".to_string(), Value::String(msg));
            out.insert("message_code".to_string(), json!(code_name));
            out.insert("success".to_string(), Value::Bool(true));

            if is_paginated_list {
                let page = obj.get("page").and_then(|v| v.as_i64()).unwrap_or(1);
                let per_page = obj.get("perPage").and_then(|v| v.as_i64()).unwrap_or(30);
                let total = obj.get("totalItems").and_then(|v| v.as_i64()).unwrap_or(0);
                let last_page = obj.get("totalPages").and_then(|v| v.as_i64()).unwrap_or(1);
                let items = obj.remove("items").unwrap_or_else(|| json!([]));
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
                let items = obj.remove("items").unwrap_or_else(|| json!([]));
                out.insert("item".to_string(), items);
            } else {
                out.insert("item".to_string(), Value::Object(obj));
            }
        }
    } else {
        out.insert("message".to_string(), json!("Success"));
        out.insert("message_code".to_string(), json!(code_name));
        out.insert("success".to_string(), Value::Bool(true));
        out.insert("item".to_string(), raw);
    }

    // Attach `debug` block in development / debug mode (matching Padi Rust Api Framework)
    if Env::is_debug() || Env::is_development() {
        let app_env = if Env::is_production() {
            "production"
        } else {
            "development"
        };
        out.insert(
            "debug".to_string(),
            json!({
                "env": app_env,
                "database": db_driver,
                "execution_time": format!("{:.2}ms", elapsed_ms),
                "queries_count": queries.len(),
                "queries": queries,
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
    let path = req.uri().path();
    if path == "/"
        || path.starts_with("/dashboard")
        || path.contains("api-collection")
        || path.ends_with(".js")
        || path.ends_with(".css")
        || path.ends_with(".png")
        || path.ends_with(".svg")
        || path.ends_with(".ico")
    {
        let mut res = next.run(req).await;
        apply_security_headers(res.headers_mut(), None);
        return res;
    }

    // Fast in-memory IP Rate Limiting (matching Padi Framework rate limiter)
    let rate_limit = Env::rate_limit_per_minute() as u64;
    let rate_info = if rate_limit > 0 {
        let ip = extract_client_ip(req.headers());
        match check_rate_limit(&ip, rate_limit) {
            Ok(info) => Some(info),
            Err(retry_after) => {
                let err_json = format_padi_json(
                    429,
                    json!({
                        "code": 429,
                        "message": "Too many requests. Please slow down."
                    }),
                    state.db.driver_name(),
                    0.0,
                    &[],
                );
                let mut res = (StatusCode::TOO_MANY_REQUESTS, axum::Json(err_json)).into_response();
                apply_security_headers(res.headers_mut(), Some((rate_limit, 0)));
                if let Ok(v) = HeaderValue::from_str(&retry_after.to_string()) {
                    res.headers_mut().insert(header::RETRY_AFTER, v);
                }
                return res;
            }
        }
    } else {
        None
    };

    let debug_enabled = Env::is_debug() || Env::is_development();
    let start = std::time::Instant::now();
    let override_match = super::db::match_override_endpoint(
        &state.db,
        req.method().as_str(),
        req.uri().path(),
    );

    let is_api_route = path.starts_with("/api") || path.ends_with("/all");
    if override_match.is_none() && !is_api_route {
        let mut res = next.run(req).await;
        apply_security_headers(res.headers_mut(), rate_info);
        return res;
    }
    let state_for_exec = state.clone();

    let exec_req = async move {
        if let Some((ep, path_params)) = override_match {
            crate::api::custom::run_override_endpoint_request(
                state_for_exec,
                ep,
                path_params,
                req,
            )
            .await
        } else {
            next.run(req).await
        }
    };

    let (mut res, captured_queries) = if debug_enabled {
        let queries = std::sync::Arc::new(std::sync::Mutex::new(Vec::with_capacity(4)));
        let q_clone = queries.clone();
        let r = REQUEST_QUERIES
            .scope(queries, async move {
                REQUEST_START.scope(start, exec_req).await
            })
            .await;
        let logs = q_clone.lock().map(|l| l.clone()).unwrap_or_default();
        (r, logs)
    } else {
        (exec_req.await, Vec::new())
    };

    if Env::response_format() == ResponseFormat::Raw && !debug_enabled {
        apply_security_headers(res.headers_mut(), rate_info);
        return res;
    }

    let status = res.status();
    if status == StatusCode::NO_CONTENT {
        apply_security_headers(res.headers_mut(), rate_info);
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
        apply_security_headers(res.headers_mut(), rate_info);
        return res;
    }

    let is_json = res
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.contains("application/json"))
        .unwrap_or(false);
    if !is_json {
        apply_security_headers(res.headers_mut(), rate_info);
        return res;
    }

    let elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;
    let db_driver = state.db.driver_name();

    let (mut parts, body) = res.into_parts();
    let Ok(bytes) = to_bytes(body, 64 * 1024 * 1024).await else {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    };

    let Ok(raw_json) = serde_json::from_slice::<Value>(&bytes) else {
        apply_security_headers(&mut parts.headers, rate_info);
        return Response::from_parts(parts, Body::from(bytes));
    };

    let formatted = format_padi_json(
        status.as_u16(),
        raw_json,
        db_driver,
        elapsed_ms,
        &captured_queries,
    );
    let out_bytes = serde_json::to_vec(&formatted).unwrap_or_else(|_| bytes.to_vec());

    parts.headers.remove(header::CONTENT_LENGTH);
    parts.headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/json; charset=utf-8"),
    );
    apply_security_headers(&mut parts.headers, rate_info);

    Response::from_parts(parts, Body::from(out_bytes))
}
