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
    authenticate_headers, check_role_rule, has_role_constraint, Claims,
};
use crate::core::db::{
    delete_custom_endpoint, get_all_custom_endpoints, get_custom_endpoint_by_path_or_id,
    save_custom_endpoint, Val,
};
use crate::core::schema::CustomEndpoint;

#[derive(Deserialize)]
pub struct TransferActionRequest {
    pub collection: String,
    pub from_id: String,
    pub to_id: String,
    pub field: String,
    pub amount: f64,
}

fn require_admin(headers: &HeaderMap) -> Result<Claims, (StatusCode, Json<Value>)> {
    let claims = authenticate_headers(headers)?.ok_or_else(|| {
        (
            StatusCode::UNAUTHORIZED,
            Json(json!({ "code": 401, "message": "Administrator authentication required." })),
        )
    })?;
    if claims.role != "admin" && claims.role != "superadmin" {
        return Err((
            StatusCode::FORBIDDEN,
            Json(json!({ "code": 403, "message": "Only administrators can manage custom endpoints." })),
        ));
    }
    Ok(claims)
}

fn check_endpoint_permission(
    rule: &str,
    auth: Option<&Claims>,
) -> Result<(), (StatusCode, Json<Value>)> {
    if let Some(c) = auth {
        if c.role == "admin" || c.role == "superadmin" {
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

    if has_role_constraint(r) && !check_role_rule(r, &claims.role) {
        return Err((
            StatusCode::FORBIDDEN,
            Json(json!({
                "code": 403,
                "message": format!("Permission denied. Your role '{}' is not authorized for this endpoint.", claims.role)
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
    if state.has_admins().await.unwrap_or(false) {
        let _ = require_admin(&headers)?;
    }

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
    if state.has_admins().await.unwrap_or(false) {
        let _ = require_admin(&headers)?;
    }

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
    if state.has_admins().await.unwrap_or(false) {
        let _ = require_admin(&headers)?;
    }

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
    if state.has_admins().await.unwrap_or(false) {
        let _ = require_admin(&headers)?;
    }

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
    if state.has_admins().await.unwrap_or(false) {
        let _ = require_admin(&headers)?;
    }

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
fn split_sql_statements(raw_sql: &str) -> Vec<String> {
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
fn compile_custom_sql(
    sql: &str,
    named_params: &Map<String, Value>,
    positional_params: &[Value],
    auth: Option<&Claims>,
) -> (String, Vec<Val>) {
    let mut out_sql = String::with_capacity(sql.len() + 16);
    let mut binds: Vec<Val> = Vec::new();
    let mut pos_idx = 0usize;

    let mut in_single = false;
    let mut in_double = false;
    let mut in_backtick = false;

    let chars: Vec<char> = sql.chars().collect();
    let len = chars.len();
    let mut i = 0usize;

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
        let c = chars[i];

        if c == '\'' && !in_double && !in_backtick {
            out_sql.push(c);
            if in_single {
                if i + 1 < len && chars[i + 1] == '\'' {
                    out_sql.push(chars[i + 1]);
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

        if c == '"' && !in_single && !in_backtick {
            in_double = !in_double;
            out_sql.push(c);
            i += 1;
            continue;
        }

        if c == '`' && !in_single && !in_double {
            in_backtick = !in_backtick;
            out_sql.push(c);
            i += 1;
            continue;
        }

        if !in_single && !in_double && !in_backtick {
            // 1. Check `@request.auth.id`, `@request.auth.email`, `@request.auth.role`
            if c == '@' {
                let rest: String = chars[i..].iter().collect();
                let lower_rest = rest.to_lowercase();
                if lower_rest.starts_with("@request.auth.id") {
                    out_sql.push('?');
                    binds.push(resolve_named("request.auth.id"));
                    i += "@request.auth.id".len();
                    continue;
                } else if lower_rest.starts_with("@request.auth.email") {
                    out_sql.push('?');
                    binds.push(resolve_named("request.auth.email"));
                    i += "@request.auth.email".len();
                    continue;
                } else if lower_rest.starts_with("@request.auth.role") {
                    out_sql.push('?');
                    binds.push(resolve_named("request.auth.role"));
                    i += "@request.auth.role".len();
                    continue;
                } else if lower_rest.starts_with("@last_insert_id") {
                    out_sql.push('?');
                    binds.push(resolve_named("last_insert_id"));
                    i += "@last_insert_id".len();
                    continue;
                } else if lower_rest.starts_with("@last_id") {
                    out_sql.push('?');
                    binds.push(resolve_named("last_id"));
                    i += "@last_id".len();
                    continue;
                }
            }

            // 2. Check `{{param_name}}`
            if c == '{' && i + 1 < len && chars[i + 1] == '{' {
                let mut end = i + 2;
                while end + 1 < len && !(chars[end] == '}' && chars[end + 1] == '}') {
                    end += 1;
                }
                if end + 1 < len && chars[end] == '}' && chars[end + 1] == '}' {
                    let param_name: String = chars[i + 2..end].iter().collect();
                    let clean_name = param_name.trim();
                    if !clean_name.is_empty() {
                        out_sql.push('?');
                        binds.push(resolve_named(clean_name));
                        i = end + 2;
                        continue;
                    }
                }
            }

            // 3. Check `:param_name` (ignoring `::` Postgres cast)
            if c == ':' {
                if i + 1 < len && chars[i + 1] == ':' {
                    out_sql.push_str("::");
                    i += 2;
                    continue;
                }
                if i + 1 < len && (chars[i + 1].is_ascii_alphabetic() || chars[i + 1] == '_') {
                    let mut end = i + 1;
                    while end < len
                        && (chars[end].is_ascii_alphanumeric()
                            || chars[end] == '_'
                            || chars[end] == '.')
                    {
                        end += 1;
                    }
                    let param_name: String = chars[i + 1..end].iter().collect();
                    out_sql.push('?');
                    binds.push(resolve_named(&param_name));
                    i = end;
                    continue;
                }
            }

            // 4. Positional `?`
            if c == '?' {
                out_sql.push('?');
                if let Some(pv) = positional_params.get(pos_idx) {
                    binds.push(Val::from(pv));
                } else {
                    binds.push(Val::Null);
                }
                pos_idx += 1;
                i += 1;
                continue;
            }
        }

        out_sql.push(c);
        i += 1;
    }

    (out_sql, binds)
}

fn is_select_statement(sql: &str) -> bool {
    sql.split_whitespace()
        .next()
        .map(|w| {
            w.eq_ignore_ascii_case("SELECT")
                || w.eq_ignore_ascii_case("WITH")
                || w.eq_ignore_ascii_case("PRAGMA")
                || w.eq_ignore_ascii_case("SHOW")
                || w.eq_ignore_ascii_case("EXPLAIN")
        })
        .unwrap_or(false)
        || sql.to_uppercase().contains(" RETURNING ")
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

    // Authenticate & check endpoint permission rule
    let auth = authenticate_headers(&headers)?;
    check_endpoint_permission(&ep.auth_rule, auth.as_ref())?;

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
        let rows = state
            .db
            .fetch_all(&compiled_sql, &binds)
            .await
            .map_err(|e| {
                (
                    StatusCode::BAD_REQUEST,
                    Json(json!({ "code": 400, "message": format!("SQL error: {}", e) })),
                )
            })?;

        let is_single = ep.single_row || is_single_row_sql(&compiled_sql) || is_single_row_sql(&statements[0]);
        if is_single {
            let first = rows.into_iter().next().unwrap_or(Value::Null);
            return Ok(Json(first));
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
    let _claims = authenticate_headers(&headers)?.ok_or_else(|| {
        (
            StatusCode::UNAUTHORIZED,
            Json(json!({ "code": 401, "message": "Authentication required." })),
        )
    })?;

    if payload.amount <= 0.0 {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({ "code": 400, "message": "Transfer amount must be greater than zero." })),
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

    let mut tx = state.db.begin().await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "code": 500, "message": e.to_string() })),
        )
    })?;

    let select_sql = format!(
        "SELECT `{}` FROM `{}` WHERE `id` = ? LIMIT 1;",
        payload.field, payload.collection
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
