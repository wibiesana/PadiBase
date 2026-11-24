//! Custom Application Endpoints & Dynamic SQL Endpoint Engine
//!
//! Allows administrators to create, edit, test, and delete custom SQL-powered REST endpoints
//! directly from the Admin Dashboard (`/dashboard/`), persisted in the `_endpoints` table
//! and served dynamically at `/api/custom/{*path}` and `/api/e/{*path}`.

use axum::{
    body::Bytes,
    extract::{Path, Query, State},
    http::{HeaderMap, Method, StatusCode},
    response::IntoResponse,
    Json,
};
use serde::Deserialize;
use serde_json::{json, Map, Value};
use std::collections::HashMap;

use crate::core::app::AppState;
use crate::core::auth::{
    authenticate_headers, check_permission, check_session, evaluate_rule_expression, require_admin,
    Claims, RecordAction,
};
use crate::core::db::{
    delete_custom_endpoint, get_all_custom_endpoints, get_collection_by_name_or_id,
    get_custom_endpoint_by_path_or_id, save_custom_endpoint, Val,
};
use crate::core::record::get_record_tx;
use crate::core::schema::{CollectionType, CustomEndpoint};

#[derive(Deserialize)]
pub struct TransferActionRequest {
    pub collection: String,
    pub from_id: String,
    pub to_id: String,
    pub field: String,
    pub amount: f64,
}

fn check_endpoint_permission(
    rule: &str,
    auth: Option<&Claims>,
    params: &Map<String, Value>,
) -> Result<(), (StatusCode, Json<Value>)> {
    if let Some(c) = auth {
        if c.is_admin() {
            return Ok(());
        }
    }

    let r = rule.trim();
    if r.is_empty()
        || r.eq_ignore_ascii_case("public")
        || r == "*"
        || r.eq_ignore_ascii_case("anyone")
        || r.eq_ignore_ascii_case("allow_all")
    {
        return Ok(());
    }

    if r.eq_ignore_ascii_case("admin") || r.eq_ignore_ascii_case("locked") {
        return Err((
            StatusCode::FORBIDDEN,
            Json(json!({
                "code": 403,
                "message": "Only administrators are allowed to access this endpoint."
            })),
        ));
    }

    let claims = auth.ok_or_else(|| {
        (
            StatusCode::UNAUTHORIZED,
            Json(json!({
                "code": 401,
                "message": "Authentication required to access this endpoint."
            })),
        )
    })?;

    let existing_ctx = if params.is_empty() { None } else { Some(params) };
    if !evaluate_rule_expression(r, Some(claims), existing_ctx) {
        return Err((
            StatusCode::FORBIDDEN,
            Json(json!({
                "code": 403,
                "message": format!(
                    "Permission denied. Your account (role '{}') does not satisfy the endpoint access rule.",
                    claims.role
                )
            })),
        ));
    }

    Ok(())
}

// ============================================================================
// CRUD MANAGEMENT FOR DYNAMIC CUSTOM ENDPOINTS (`/api/endpoints`)
// ============================================================================

/// List all dynamic custom SQL endpoints (`GET /api/endpoints`)
pub async fn list_endpoints(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, (StatusCode, Json<Value>)> {
    require_admin(&state, &headers).await?;

    let items = get_all_custom_endpoints(&state.db).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "code": 500, "message": e.to_string() })),
        )
    })?;

    Ok(Json(json!(items)))
}

/// Get single custom endpoint by ID or path (`GET /api/endpoints/{id_or_path}`)
pub async fn get_endpoint(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id_or_path): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, Json<Value>)> {
    require_admin(&state, &headers).await?;

    let ep = get_custom_endpoint_by_path_or_id(&state.db, &id_or_path)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "code": 500, "message": e.to_string() })),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(json!({ "code": 404, "message": "Custom endpoint not found." })),
            )
        })?;

    Ok(Json(json!(ep)))
}

/// Create a new dynamic custom SQL endpoint (`POST /api/endpoints`)
pub async fn create_endpoint(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<CustomEndpoint>,
) -> Result<impl IntoResponse, (StatusCode, Json<Value>)> {
    require_admin(&state, &headers).await?;

    let saved = save_custom_endpoint(&state.db, &payload).await.map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            Json(json!({ "code": 400, "message": e.to_string() })),
        )
    })?;

    Ok((StatusCode::CREATED, Json(json!(saved))))
}

/// Update an existing dynamic custom SQL endpoint (`PATCH/PUT /api/endpoints/{id_or_path}`)
pub async fn update_endpoint(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id_or_path): Path<String>,
    Json(mut payload): Json<CustomEndpoint>,
) -> Result<impl IntoResponse, (StatusCode, Json<Value>)> {
    require_admin(&state, &headers).await?;

    let existing = get_custom_endpoint_by_path_or_id(&state.db, &id_or_path)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "code": 500, "message": e.to_string() })),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(json!({ "code": 404, "message": "Custom endpoint not found." })),
            )
        })?;

    payload.id = existing.id;
    payload.created = existing.created;

    let saved = save_custom_endpoint(&state.db, &payload).await.map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            Json(json!({ "code": 400, "message": e.to_string() })),
        )
    })?;

    Ok(Json(json!(saved)))
}

/// Delete a dynamic custom SQL endpoint (`DELETE /api/endpoints/{id_or_path}`)
pub async fn delete_endpoint(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id_or_path): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, Json<Value>)> {
    require_admin(&state, &headers).await?;

    let deleted = delete_custom_endpoint(&state.db, &id_or_path)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "code": 500, "message": e.to_string() })),
            )
        })?;

    if !deleted {
        return Err((
            StatusCode::NOT_FOUND,
            Json(json!({ "code": 404, "message": "Custom endpoint not found." })),
        ));
    }

    Ok(Json(json!({ "deleted": true })))
}

// ============================================================================
// DYNAMIC SQL PARAMETER BINDING & EXECUTION ENGINE (`/api/custom/{*path}`)
// ============================================================================

fn param_value_to_val(key: &str, v: &Value) -> Val {
    match v {
        Value::Null => Val::Null,
        Value::Bool(b) => Val::Bool(*b),
        Value::Number(n) => n
            .as_i64()
            .map(Val::Int)
            .unwrap_or_else(|| Val::Float(n.as_f64().unwrap_or(0.0))),
        Value::String(s) => {
            let k_lower = key.to_lowercase();
            if k_lower == "limit"
                || k_lower == "offset"
                || k_lower == "page"
                || k_lower == "per_page"
                || k_lower == "perpage"
            {
                if let Ok(num) = s.trim().parse::<i64>() {
                    return Val::Int(num);
                }
            }
            Val::Str(s.clone())
        }
        other => Val::Str(other.to_string()),
    }
}

/// Split multiple SQL statements by `;` while ignoring semicolons inside quotes/backticks.
pub fn split_sql_statements(raw_sql: &str) -> Vec<String> {
    let mut stmts = Vec::new();
    let mut current = String::new();
    let mut in_single = false;
    let mut in_double = false;
    let mut in_backtick = false;
    let mut chars = raw_sql.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '\'' && !in_double && !in_backtick {
            current.push(c);
            if in_single {
                if chars.peek() == Some(&'\'') {
                    if let Some(esc) = chars.next() {
                        current.push(esc);
                    }
                } else {
                    in_single = false;
                }
            } else {
                in_single = true;
            }
        } else if c == '"' && !in_single && !in_backtick {
            in_double = !in_double;
            current.push(c);
        } else if c == '`' && !in_single && !in_double {
            in_backtick = !in_backtick;
            current.push(c);
        } else if c == ';' && !in_single && !in_double && !in_backtick {
            let trimmed = current.trim();
            if !trimmed.is_empty() {
                stmts.push(trimmed.to_string());
            }
            current.clear();
        } else {
            current.push(c);
        }
    }

    let trimmed = current.trim();
    if !trimmed.is_empty() {
        stmts.push(trimmed.to_string());
    }
    stmts
}

/// Compile a custom SQL query supporting:
/// - `@request.auth.id`, `@request.auth.email`, `@request.auth.role`
/// - Named parameters `:param_name` or `{{param_name}}`
/// - Positional parameters `?`
/// Uses zero-allocation byte slice scanning on the hot path.
pub fn compile_custom_sql(
    sql: &str,
    named_params: &Map<String, Value>,
    positional_params: &[Value],
    auth: Option<&Claims>,
) -> (String, Vec<Val>) {
    let bytes = sql.as_bytes();
    let len = bytes.len();
    let mut out_sql = String::with_capacity(len + 16);
    let mut binds: Vec<Val> = Vec::with_capacity(8);
    let mut pos_idx = 0usize;

    let mut in_single = false;
    let mut in_double = false;
    let mut in_backtick = false;

    let mut i = 0usize;
    let mut seg_start = 0usize;

    let resolve_named = |name: &str| -> Val {
        match name {
            "auth_id" | "request.auth.id" => auth
                .map(|c| Val::from(c.sub.as_str()))
                .unwrap_or(Val::Null),
            "auth_email" | "request.auth.email" => auth
                .map(|c| Val::from(c.email.as_str()))
                .unwrap_or(Val::Null),
            "auth_role" | "request.auth.role" => auth
                .map(|c| Val::from(c.role.as_str()))
                .unwrap_or(Val::Null),
            _ => {
                if let Some(v) = named_params.get(name) {
                    return param_value_to_val(name, v);
                }
                for (k, v) in named_params {
                    if k.eq_ignore_ascii_case(name) {
                        return param_value_to_val(k, v);
                    }
                }
                Val::Null
            }
        }
    };

    while i < len {
        let b = bytes[i];

        if b == b'\'' && !in_double && !in_backtick {
            if in_single {
                if i + 1 < len && bytes[i + 1] == b'\'' {
                    i += 2;
                    continue;
                } else {
                    in_single = false;
                }
            } else {
                in_single = true;
            }
            i += 1;
            continue;
        }

        if b == b'"' && !in_single && !in_backtick {
            in_double = !in_double;
            i += 1;
            continue;
        }

        if b == b'`' && !in_single && !in_double {
            in_backtick = !in_backtick;
            i += 1;
            continue;
        }

        if !in_single && !in_double && !in_backtick {
            // 1. Check `@request.auth.id`, `@request.auth.email`, `@request.auth.role`, `@last_insert_id`, `@last_id`
            if b == b'@' {
                let rest = &sql[i..];
                let matched_token = if rest.len() >= 16 && rest[..16].eq_ignore_ascii_case("@request.auth.id") {
                    Some((16, "request.auth.id"))
                } else if rest.len() >= 19 && rest[..19].eq_ignore_ascii_case("@request.auth.email") {
                    Some((19, "request.auth.email"))
                } else if rest.len() >= 18 && rest[..18].eq_ignore_ascii_case("@request.auth.role") {
                    Some((18, "request.auth.role"))
                } else if rest.len() >= 15 && rest[..15].eq_ignore_ascii_case("@last_insert_id") {
                    Some((15, "last_insert_id"))
                } else if rest.len() >= 8 && rest[..8].eq_ignore_ascii_case("@last_id") {
                    Some((8, "last_id"))
                } else {
                    None
                };

                if let Some((tok_len, key)) = matched_token {
                    out_sql.push_str(&sql[seg_start..i]);
                    out_sql.push('?');
                    binds.push(resolve_named(key));
                    i += tok_len;
                    seg_start = i;
                    continue;
                }
            }

            // 2. Check `{{param_name}}`
            if b == b'{' && i + 1 < len && bytes[i + 1] == b'{' {
                let mut end = i + 2;
                while end + 1 < len && !(bytes[end] == b'}' && bytes[end + 1] == b'}') {
                    end += 1;
                }
                if end + 1 < len && bytes[end] == b'}' && bytes[end + 1] == b'}' {
                    let clean_name = sql[i + 2..end].trim();
                    if !clean_name.is_empty() {
                        out_sql.push_str(&sql[seg_start..i]);
                        out_sql.push('?');
                        binds.push(resolve_named(clean_name));
                        i = end + 2;
                        seg_start = i;
                        continue;
                    }
                }
            }

            // 3. Check `:param_name` (ignoring `::` Postgres cast)
            if b == b':' {
                if i + 1 < len && bytes[i + 1] == b':' {
                    i += 2;
                    continue;
                }
                if i + 1 < len && (bytes[i + 1].is_ascii_alphabetic() || bytes[i + 1] == b'_') {
                    let mut end = i + 1;
                    while end < len
                        && (bytes[end].is_ascii_alphanumeric()
                            || bytes[end] == b'_'
                            || bytes[end] == b'.')
                    {
                        end += 1;
                    }
                    let param_name = &sql[i + 1..end];
                    out_sql.push_str(&sql[seg_start..i]);
                    out_sql.push('?');
                    let mut val = resolve_named(param_name);
                    if val == Val::Null && pos_idx < positional_params.len() {
                        val = param_value_to_val(param_name, &positional_params[pos_idx]);
                        pos_idx += 1;
                    }
                    binds.push(val);
                    i = end;
                    seg_start = i;
                    continue;
                }
            }

            // 4. Positional `?`
            if b == b'?' {
                out_sql.push_str(&sql[seg_start..i]);
                out_sql.push('?');
                if let Some(pv) = positional_params.get(pos_idx) {
                    binds.push(Val::from(pv));
                } else {
                    binds.push(Val::Null);
                }
                pos_idx += 1;
                i += 1;
                seg_start = i;
                continue;
            }
        }

        i += 1;
    }

    if seg_start < len {
        out_sql.push_str(&sql[seg_start..len]);
    }

    (out_sql, binds)
}

pub fn is_select_statement(sql: &str) -> bool {
    let uncommented = strip_sql_comments(sql);
    uncommented
        .split_whitespace()
        .next()
        .map(|w| {
            w.eq_ignore_ascii_case("SELECT")
                || w.eq_ignore_ascii_case("WITH")
                || w.eq_ignore_ascii_case("PRAGMA")
                || w.eq_ignore_ascii_case("SHOW")
                || w.eq_ignore_ascii_case("EXPLAIN")
        })
        .unwrap_or(false)
        || uncommented.to_uppercase().contains(" RETURNING ")
}

fn strip_sql_comments(sql: &str) -> String {
    let mut result = String::with_capacity(sql.len());
    let mut chars = sql.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '-' && chars.peek() == Some(&'-') {
            chars.next();
            while let Some(&nc) = chars.peek() {
                chars.next();
                if nc == '\n' {
                    result.push('\n');
                    break;
                }
            }
        } else if c == '/' && chars.peek() == Some(&'*') {
            chars.next();
            while let Some(nc) = chars.next() {
                if nc == '*' && chars.peek() == Some(&'/') {
                    chars.next();
                    break;
                }
            }
        } else {
            result.push(c);
        }
    }
    result
}

pub fn is_single_row_sql(sql: &str) -> bool {
    let uncommented = strip_sql_comments(sql);
    let cleaned = uncommented.replace([';', ',', '\r', '\n', '\t', '(', ')'], " ");
    let words: Vec<&str> = cleaned.split_whitespace().collect();
    for i in 0..words.len() {
        if words[i].eq_ignore_ascii_case("LIMIT") {
            // Case 1: LIMIT 1 ...
            if i + 1 < words.len() && words[i + 1] == "1" {
                return true;
            }
            // Case 2: LIMIT <offset> 1 ... (from MySQL syntax `LIMIT offset, 1`)
            if i + 2 < words.len()
                && words[i + 1].chars().all(|c| c.is_ascii_digit())
                && words[i + 2] == "1"
            {
                return true;
            }
        }
    }
    false
}

/// Dynamic Custom Endpoint Dispatcher (`ANY /api/custom/{*path}` and `ANY /api/e/{*path}`)
pub async fn execute_dynamic_endpoint(
    State(state): State<AppState>,
    method: Method,
    headers: HeaderMap,
    Path(raw_path): Path<String>,
    Query(query_params): Query<HashMap<String, String>>,
    body_bytes: Bytes,
) -> Result<impl IntoResponse, (StatusCode, Json<Value>)> {
    let clean_path = raw_path.trim().trim_matches('/');

    // Lookup dynamic endpoint in RAM cache / `_endpoints` table
    let ep_opt = get_custom_endpoint_by_path_or_id(&state.db, clean_path)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "code": 500, "message": e.to_string() })),
            )
        })?;

    // Fallback to built-in `/api/custom/transfer` if no user-defined `transfer` endpoint overrides it
    let Some(ep) = ep_opt else {
        if clean_path.eq_ignore_ascii_case("transfer") && method == Method::POST {
            let payload: TransferActionRequest =
                serde_json::from_slice(&body_bytes).map_err(|e| {
                    (
                        StatusCode::BAD_REQUEST,
                        Json(json!({ "code": 400, "message": format!("Invalid transfer payload: {}", e) })),
                    )
                })?;
            return handle_atomic_transfer(state, headers, payload).await;
        }

        return Err((
            StatusCode::NOT_FOUND,
            Json(json!({
                "code": 404,
                "message": format!("Custom endpoint '/api/custom/{}' was not found.", clean_path)
            })),
        ));
    };

    execute_custom_endpoint_inner(state, ep, method, headers, query_params, body_bytes).await
}

/// Intercepts and executes an overridden built-in endpoint from `padi_response_middleware`.
pub async fn run_override_endpoint_request(
    state: AppState,
    ep: CustomEndpoint,
    path_params: HashMap<String, String>,
    req: axum::extract::Request,
) -> axum::response::Response {
    let method = req.method().clone();
    let headers = req.headers().clone();
    let query_str = req.uri().query().unwrap_or("");
    let mut query_params: HashMap<String, String> = HashMap::new();
    for pair in query_str.split('&') {
        if let Some((k, v)) = pair.split_once('=') {
            if !k.is_empty() {
                query_params.insert(k.to_string(), v.to_string());
            }
        } else if !pair.is_empty() {
            query_params.insert(pair.to_string(), String::new());
        }
    }
    for (k, v) in path_params {
        query_params.insert(k, v);
    }

    let body_bytes = axum::body::to_bytes(req.into_body(), 16 * 1024 * 1024)
        .await
        .unwrap_or_default();

    match execute_custom_endpoint_inner(state, ep, method, headers, query_params, body_bytes).await {
        Ok(res) => res.into_response(),
        Err(err) => err.into_response(),
    }
}

pub fn resolve_pagination_params(
    sql: &str,
    named_params: &mut Map<String, Value>,
) -> Option<(i64, i64)> {
    // Support `order` as alias for `sort_dir` (and vice versa) for dynamic ORDER BY
    if let Some(order_val) = named_params.get("order").cloned() {
        if !named_params.contains_key("sort_dir") {
            named_params.insert("sort_dir".to_string(), order_val);
        }
    } else if let Some(dir_val) = named_params.get("sort_dir").cloned() {
        named_params.insert("order".to_string(), dir_val);
    }

    let parse_i64 = |v: &Value| -> Option<i64> {
        match v {
            Value::Number(n) => n.as_i64(),
            Value::String(s) => s.trim().parse::<i64>().ok(),
            _ => None,
        }
    };

    let has_explicit_page = named_params.contains_key("page")
        || named_params.contains_key("perPage")
        || named_params.contains_key("per_page");
    let sql_lower = sql.to_lowercase();
    let has_sql_pagination = sql_lower.contains(":limit") && sql_lower.contains(":offset");

    if !has_explicit_page && !has_sql_pagination {
        // Non-paginated query that still exposes `:limit` (e.g. /api/banks/all?limit=5000)
        if sql_lower.contains(":limit") {
            let safe_limit = named_params
                .get("limit")
                .and_then(parse_i64)
                .unwrap_or(MAX_UNPAGINATED_CUSTOM_ROWS as i64)
                .clamp(1, MAX_UNPAGINATED_CUSTOM_ROWS as i64);
            named_params.insert("limit".to_string(), json!(safe_limit));
        }
        return None;
    }

    let per_page = named_params
        .get("perPage")
        .or_else(|| named_params.get("per_page"))
        .or_else(|| named_params.get("limit"))
        .and_then(parse_i64)
        .unwrap_or(30)
        .clamp(1, 500);

    let page = if let Some(p) = named_params.get("page").and_then(parse_i64) {
        p.max(1)
    } else if let Some(off) = named_params.get("offset").and_then(parse_i64) {
        (off.max(0) / per_page) + 1
    } else {
        1
    };

    if !named_params.contains_key("limit") {
        named_params.insert("limit".to_string(), json!(per_page));
    }
    if !named_params.contains_key("offset") {
        named_params.insert("offset".to_string(), json!((page - 1) * per_page));
    }

    Some((page, per_page))
}

pub fn strip_trailing_limit_offset(sql: &str) -> String {
    let trimmed = sql.trim().trim_end_matches(';').trim();
    // Strip trailing `LIMIT <expr> [OFFSET <expr>]` or `LIMIT <expr>, <expr>` at depth 0
    let bytes = trimmed.as_bytes();
    let mut depth: i32 = 0;
    let mut in_single = false;
    let mut in_double = false;
    let mut last_limit_idx: Option<usize> = None;

    let mut i = 0;
    while i < bytes.len() {
        let b = bytes[i];
        if b == b'\'' && !in_double {
            in_single = !in_single;
        } else if b == b'"' && !in_single {
            in_double = !in_double;
        } else if !in_single && !in_double {
            if b == b'(' {
                depth += 1;
            } else if b == b')' && depth > 0 {
                depth -= 1;
            } else if depth == 0 && i + 5 <= bytes.len() {
                let slice = &trimmed[i..];
                if slice.len() >= 5
                    && slice[..5].eq_ignore_ascii_case("limit")
                    && (i == 0 || bytes[i - 1].is_ascii_whitespace() || bytes[i - 1] == b')')
                    && (i + 5 == bytes.len() || bytes[i + 5].is_ascii_whitespace())
                {
                    last_limit_idx = Some(i);
                }
            }
        }
        i += 1;
    }

    match last_limit_idx {
        Some(idx) => trimmed[..idx].trim().to_string(),
        None => trimmed.to_string(),
    }
}

pub const MAX_UNPAGINATED_CUSTOM_ROWS: usize = 5000;

pub fn apply_unpaginated_row_cap(compiled_sql: &str, max_rows: usize) -> String {
    let trimmed = compiled_sql.trim().trim_end_matches(';').trim();
    if !is_select_statement(trimmed) {
        return compiled_sql.to_string();
    }
    let stripped = strip_trailing_limit_offset(trimmed);
    if stripped.len() == trimmed.len() {
        format!("{} LIMIT {}", trimmed, max_rows)
    } else {
        compiled_sql.to_string()
    }
}

pub async fn count_paginated_select(
    state: &AppState,
    raw_sql: &str,
    named_params: &Map<String, Value>,
    positional_params: &[Value],
    auth: Option<&Claims>,
) -> Option<i64> {
    let base_sql = strip_trailing_limit_offset(raw_sql);
    if !is_select_statement(&base_sql) {
        return None;
    }
    let count_sql_raw = format!("SELECT COUNT(*) AS _padi_cnt FROM ({}) AS _padi_sub", base_sql);
    let (compiled_count, count_binds) =
        compile_custom_sql(&count_sql_raw, named_params, positional_params, auth);
    let row = state.db.fetch_optional(&compiled_count, &count_binds).await.ok()??;
    let val = row.get("_padi_cnt")?;
    match val {
        Value::Number(n) => n.as_i64(),
        Value::String(s) => s.trim().parse::<i64>().ok(),
        _ => None,
    }
}

async fn execute_custom_endpoint_inner(
    state: AppState,
    ep: CustomEndpoint,
    method: Method,
    headers: HeaderMap,
    query_params: HashMap<String, String>,
    body_bytes: Bytes,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    // Check HTTP Method match (unless endpoint is configured as "ANY" or "*")
    let ep_method = ep.method.trim().to_uppercase();
    if ep_method != "ANY" && ep_method != "*" && ep_method != method.as_str() {
        return Err((
            StatusCode::METHOD_NOT_ALLOWED,
            Json(json!({
                "code": 405,
                "message": format!("Endpoint '{}' requires HTTP {} (received {}).", ep.path, ep_method, method.as_str())
            })),
        ));
    }

    // Authenticate & check session
    let auth = authenticate_headers(&headers)?;
    check_session(&state, &headers).await?;

    // Merge query parameters + JSON body parameters
    let mut named_params = Map::new();
    let mut positional_params: Vec<Value> = Vec::new();

    for (k, v) in query_params {
        if k == "params" {
            for part in v.split(',') {
                positional_params.push(Value::String(part.trim().to_string()));
            }
        }
        named_params.insert(k, Value::String(v));
    }

    if !body_bytes.is_empty() {
        if let Ok(Value::Object(obj)) = serde_json::from_slice::<Value>(&body_bytes) {
            if let Some(Value::Array(arr)) = obj.get("params") {
                positional_params = arr.clone();
            }
            for (k, v) in obj {
                named_params.insert(k, v);
            }
        }
    }

    let pagination_ctx = if !ep.single_row {
        resolve_pagination_params(&ep.sql, &mut named_params)
    } else {
        None
    };

    // If an ownership rule is defined (e.g. `user_id = @request.auth.id`), auto-populate missing
    // owner parameter from the authenticated user's ID and verify permission against request params.
    if let Some(ref c) = auth {
        let lower_rule = ep.auth_rule.to_lowercase();
        for owner_col in ["user_id", "created_by", "author_id", "owner_id"] {
            if lower_rule.contains(&format!("{} = @request.auth.id", owner_col))
                || lower_rule.contains(&format!("{}=@request.auth.id", owner_col))
            {
                if !named_params.contains_key(owner_col) {
                    named_params.insert(owner_col.to_string(), Value::String(c.sub.clone()));
                }
            }
        }
    }

    check_endpoint_permission(&ep.auth_rule, auth.as_ref(), &named_params)?;

    let non_comment_content: String = ep
        .sql
        .lines()
        .filter(|line| !line.trim().starts_with("--"))
        .collect::<Vec<_>>()
        .join("\n");
    let trimmed_sql = non_comment_content.trim();

    // 1. Direct Static JSON Response (e.g. { "message": "hello" } or ["hello", "world"])
    if (trimmed_sql.starts_with('{') && trimmed_sql.ends_with('}'))
        || (trimmed_sql.starts_with('[') && trimmed_sql.ends_with(']'))
    {
        if let Ok(json_val) = serde_json::from_str::<Value>(trimmed_sql) {
            return Ok(Json(json_val));
        }
    }

    // 2. Direct Quoted String Literal (e.g. 'hello' or "hello")
    if (trimmed_sql.starts_with('\'') && trimmed_sql.ends_with('\'') && trimmed_sql.len() >= 2)
        || (trimmed_sql.starts_with('"') && trimmed_sql.ends_with('"') && trimmed_sql.len() >= 2)
    {
        let inner = &trimmed_sql[1..trimmed_sql.len() - 1];
        return Ok(Json(json!({ "message": inner })));
    }

    // 3. Direct Plain Text / Mock String without SQL keywords (e.g. "hello", "pong", "OK")
    let first_word = trimmed_sql
        .split_whitespace()
        .next()
        .unwrap_or("")
        .to_uppercase();

    let sql_keywords = [
        "SELECT", "INSERT", "UPDATE", "DELETE", "WITH", "PRAGMA", "SHOW", "EXPLAIN",
        "CREATE", "DROP", "ALTER", "REPLACE",
    ];

    if !trimmed_sql.is_empty() && !sql_keywords.contains(&first_word.as_str()) && !trimmed_sql.contains(';') {
        // Plain text scalar message without needing SQL syntax
        return Ok(Json(json!({ "message": trimmed_sql })));
    }

    let statements = split_sql_statements(&ep.sql);
    if statements.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({ "code": 400, "message": "Endpoint SQL query is empty." })),
        ));
    }

    // Run inside a transaction if `ep.transaction` is true OR if there are multiple SQL statements
    if ep.transaction || statements.len() > 1 {
        let mut tx = state.db.begin().await.map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "code": 500, "message": format!("Failed to begin transaction: {}", e) })),
            )
        })?;

        let mut last_rows: Option<Vec<Value>> = None;
        let mut total_affected = 0u64;
        let mut last_insert_id: Option<i64> = None;
        let mut dynamic_params = named_params.clone();

        for (idx, stmt) in statements.iter().enumerate() {
            if let Some(lid) = last_insert_id {
                dynamic_params.insert("last_insert_id".to_string(), json!(lid));
                dynamic_params.insert("last_id".to_string(), json!(lid));
            }

            let (compiled_sql, binds) =
                compile_custom_sql(stmt, &dynamic_params, &positional_params, auth.as_ref());

            if is_select_statement(&compiled_sql) {
                match tx.fetch_all(&state.db, &compiled_sql, &binds).await {
                    Ok(rows) => {
                        last_rows = Some(rows);
                    }
                    Err(e) => {
                        let _ = tx.rollback().await;
                        return Err((
                            StatusCode::BAD_REQUEST,
                            Json(json!({
                                "code": 400,
                                "message": format!("SQL error in statement {}: {}", idx + 1, e)
                            })),
                        ));
                    }
                }
            } else {
                match tx.execute(&state.db, &compiled_sql, &binds).await {
                    Ok((affected, lid)) => {
                        total_affected += affected;
                        if lid.is_some() {
                            last_insert_id = lid;
                        }
                    }
                    Err(e) => {
                        let _ = tx.rollback().await;
                        return Err((
                            StatusCode::BAD_REQUEST,
                            Json(json!({
                                "code": 400,
                                "message": format!("SQL error in statement {}: {}", idx + 1, e)
                            })),
                        ));
                    }
                }
            }
        }

        tx.commit().await.map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "code": 500, "message": format!("Transaction commit failed: {}", e) })),
            )
        })?;

        if let Some(rows) = last_rows {
            let last_stmt = statements.last().map(|s| s.as_str()).unwrap_or("");
            if ep.single_row || is_single_row_sql(last_stmt) {
                let first = rows.into_iter().next().unwrap_or(Value::Null);
                return Ok(Json(first));
            }
            return Ok(Json(json!({
                "items": rows,
                "totalItems": rows.len()
            })));
        }

        return Ok(Json(json!({
            "committed": true,
            "rows_affected": total_affected,
            "last_insert_id": last_insert_id
        })));
    }

    // Single statement execution
    let (compiled_sql, binds) =
        compile_custom_sql(&statements[0], &named_params, &positional_params, auth.as_ref());

    if is_select_statement(&compiled_sql) {
        let is_single = ep.single_row || is_single_row_sql(&compiled_sql) || is_single_row_sql(&statements[0]);
        if is_single {
            let first = state
                .db
                .fetch_optional(&compiled_sql, &binds)
                .await
                .map_err(|e| {
                    (
                        StatusCode::BAD_REQUEST,
                        Json(json!({ "code": 400, "message": format!("SQL error: {}", e) })),
                    )
                })?
                .unwrap_or(Value::Null);
            return Ok(Json(first));
        }

        let effective_sql = if pagination_ctx.is_none() {
            apply_unpaginated_row_cap(&compiled_sql, MAX_UNPAGINATED_CUSTOM_ROWS)
        } else {
            compiled_sql.clone()
        };

        let rows = state
            .db
            .fetch_all(&effective_sql, &binds)
            .await
            .map_err(|e| {
                (
                    StatusCode::BAD_REQUEST,
                    Json(json!({ "code": 400, "message": format!("SQL error: {}", e) })),
                )
            })?;

        if let Some((page, per_page)) = pagination_ctx {
            let total_items = count_paginated_select(
                &state,
                &statements[0],
                &named_params,
                &positional_params,
                auth.as_ref(),
            )
            .await
            .unwrap_or(rows.len() as i64);
            let total_pages = ((total_items + per_page - 1) / per_page).max(1);
            return Ok(Json(json!({
                "page": page,
                "perPage": per_page,
                "totalItems": total_items,
                "totalPages": total_pages,
                "items": rows
            })));
        }

        Ok(Json(json!({
            "items": rows,
            "totalItems": rows.len()
        })))
    } else {
        let (rows_affected, last_insert_id) = state
            .db
            .execute(&compiled_sql, &binds)
            .await
            .map_err(|e| {
                (
                    StatusCode::BAD_REQUEST,
                    Json(json!({ "code": 400, "message": format!("SQL error: {}", e) })),
                )
            })?;

        Ok(Json(json!({
            "rows_affected": rows_affected,
            "last_insert_id": last_insert_id
        })))
    }
}

/// Built-in transactional transfer handler (`POST /api/custom/transfer`)
pub async fn atomic_transfer(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<TransferActionRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<Value>)> {
    handle_atomic_transfer(state, headers, payload).await
}

async fn handle_atomic_transfer(
    state: AppState,
    headers: HeaderMap,
    payload: TransferActionRequest,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let claims = authenticate_headers(&headers)?.ok_or_else(|| {
        (
            StatusCode::UNAUTHORIZED,
            Json(json!({ "code": 401, "message": "Authentication required." })),
        )
    })?;
    check_session(&state, &headers).await?;

    if !payload.amount.is_finite() || payload.amount <= 0.0 {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({ "code": 400, "message": "Transfer amount must be a finite number greater than zero." })),
        ));
    }
    if payload.from_id == payload.to_id {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({ "code": 400, "message": "Source and destination records must be different." })),
        ));
    }

    if !crate::core::db::is_safe_identifier(&payload.collection)
        || !crate::core::db::is_safe_identifier(&payload.field)
        || payload.collection.starts_with('_')
    {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({ "code": 400, "message": "Invalid collection or field identifier." })),
        ));
    }

    let col = get_collection_by_name_or_id(&state.db, &payload.collection)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "code": 500, "message": e.to_string() })),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(json!({ "code": 404, "message": "Collection not found." })),
            )
        })?;

    let mut tx = state.db.begin().await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "code": 500, "message": e.to_string() })),
        )
    })?;

    let source_rec = get_record_tx(&mut tx, &state.db, &col, &payload.from_id)
        .await
        .map_err(|e| {
            (
                StatusCode::BAD_REQUEST,
                Json(json!({ "code": 400, "message": e.to_string() })),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(json!({ "code": 404, "message": "Source record not found." })),
            )
        })?;

    // Verify the caller owns / is allowed to debit the source record.
    if !claims.is_admin() {
        check_permission(
            RecordAction::Update,
            &col,
            Some(&claims),
            Some(&source_rec.data),
        )?;
        let owns_source = if col.collection_type == CollectionType::Auth {
            source_rec.id == claims.sub
        } else if let Some(owner_uid) = source_rec
            .data
            .get("user_id")
            .or_else(|| source_rec.data.get("created_by"))
            .or_else(|| source_rec.data.get("owner_id"))
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
        {
            owner_uid == claims.sub
        } else {
            true
        };
        if !owns_source {
            let _ = tx.rollback().await;
            return Err((
                StatusCode::FORBIDDEN,
                Json(json!({ "code": 403, "message": "You can only transfer from your own account/record." })),
            ));
        }
    }

    let select_sql = format!(
        "SELECT `{}` FROM `{}` WHERE `id` = ? LIMIT 1;",
        payload.field, col.name
    );
    let source_row = tx
        .fetch_optional(&state.db, &select_sql, &[Val::from(&payload.from_id)])
        .await
        .map_err(|e| {
            (
                StatusCode::BAD_REQUEST,
                Json(json!({ "code": 400, "message": e.to_string() })),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(json!({ "code": 404, "message": "Source record not found." })),
            )
        })?;

    let current_balance = source_row
        .get(&payload.field)
        .and_then(|v| v.as_f64().or_else(|| v.as_i64().map(|n| n as f64)))
        .unwrap_or(0.0);

    if current_balance < payload.amount {
        let _ = tx.rollback().await;
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({
                "code": 400,
                "message": format!("Insufficient '{}': available {}, required {}", payload.field, current_balance, payload.amount)
            })),
        ));
    }

    let deduct_sql = format!(
        "UPDATE `{}` SET `{}` = `{}` - ? WHERE `id` = ?;",
        payload.collection, payload.field, payload.field
    );
    let (deducted, _) = tx
        .execute(
            &state.db,
            &deduct_sql,
            &[Val::from(payload.amount), Val::from(&payload.from_id)],
        )
        .await
        .map_err(|e| {
            (
                StatusCode::BAD_REQUEST,
                Json(json!({ "code": 400, "message": e.to_string() })),
            )
        })?;

    if deducted == 0 {
        let _ = tx.rollback().await;
        return Err((
            StatusCode::NOT_FOUND,
            Json(json!({ "code": 404, "message": "Source record not found." })),
        ));
    }

    let credit_sql = format!(
        "UPDATE `{}` SET `{}` = `{}` + ? WHERE `id` = ?;",
        payload.collection, payload.field, payload.field
    );
    let (credited, _) = tx
        .execute(
            &state.db,
            &credit_sql,
            &[Val::from(payload.amount), Val::from(&payload.to_id)],
        )
        .await
        .map_err(|e| {
            (
                StatusCode::BAD_REQUEST,
                Json(json!({ "code": 400, "message": e.to_string() })),
            )
        })?;

    if credited == 0 {
        let _ = tx.rollback().await;
        return Err((
            StatusCode::NOT_FOUND,
            Json(json!({ "code": 404, "message": "Destination record not found. Transaction rolled back." })),
        ));
    }

    tx.commit().await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "code": 500, "message": e.to_string() })),
        )
    })?;

    Ok(Json(json!({
        "transferred": true,
        "collection": payload.collection,
        "from_id": payload.from_id,
        "to_id": payload.to_id,
        "field": payload.field,
        "amount": payload.amount
    })))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_single_row_sql() {
        assert!(is_single_row_sql("SELECT * FROM users LIMIT 1;"));
        assert!(is_single_row_sql("SELECT * FROM users LIMIT 1"));
        assert!(is_single_row_sql("SELECT * FROM users limit 1;"));
        assert!(is_single_row_sql("SELECT * FROM users LIMIT 0, 1;"));
        assert!(is_single_row_sql("SELECT * FROM users LIMIT 0,1;"));
        assert!(is_single_row_sql("SELECT * FROM users LIMIT 1 OFFSET 0;"));
        assert!(is_single_row_sql("SELECT * FROM users WHERE id = :id LIMIT 1"));
        assert!(is_single_row_sql("-- some comment\nSELECT * FROM users LIMIT 1;"));

        // Should NOT be single row
        assert!(!is_single_row_sql("SELECT * FROM users;"));
        assert!(!is_single_row_sql("SELECT * FROM users LIMIT 10;"));
        assert!(!is_single_row_sql("SELECT * FROM users LIMIT 100;"));
        assert!(!is_single_row_sql("SELECT * FROM users LIMIT 2;"));
        assert!(!is_single_row_sql("-- LIMIT 1 in comment\nSELECT * FROM users;"));
    }
}
