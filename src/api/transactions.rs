use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    Json,
};
use serde::Deserialize;
use serde_json::{json, Map, Value};

use crate::core::app::AppState;
use crate::core::auth::{authenticate_headers, check_permission, RecordAction};
use crate::core::db::{get_collection_by_name_or_id, Val};
use crate::core::realtime::RealtimeMessage;
use crate::core::record::{create_record_tx, delete_record_tx, get_record_tx, update_record_tx};

#[derive(Deserialize)]
pub struct TransactionOperation {
    /// Operation type: `"create"`, `"update"`, `"delete"`, or `"sql"` (admin only)
    pub action: String,
    /// Target collection name or ID (required for `create`, `update`, `delete`)
    #[serde(default)]
    pub collection: Option<String>,
    /// Target record ID (required for `update` and `delete`)
    #[serde(default)]
    pub id: Option<String>,
    /// Record payload (for `create` and `update`)
    #[serde(default)]
    pub data: Option<Map<String, Value>>,
    /// Raw SQL statement (only allowed when authenticated as `admin` and `action == "sql"`)
    #[serde(default)]
    pub sql: Option<String>,
    /// Bind parameters for raw SQL statement
    #[serde(default)]
    pub params: Vec<Value>,
}

#[derive(Deserialize)]
pub struct TransactionRequest {
    #[serde(alias = "requests", alias = "batch")]
    pub operations: Vec<TransactionOperation>,
}

#[derive(Deserialize)]
pub struct RawSqlRequest {
    pub sql: String,
    #[serde(default)]
    pub params: Vec<Value>,
    /// Optional flag to run inside a transaction
    #[serde(default)]
    pub transaction: bool,
}

/// Execute multiple operations (`create`, `update`, `delete`, or admin `sql`) atomically in a single DB transaction.
/// If any step fails validation, permission check, or SQL execution, the entire transaction rolls back automatically.
///
/// Endpoint: `POST /api/transactions` or `POST /api/batch`
pub async fn execute_transaction(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<TransactionRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<Value>)> {
    let auth = authenticate_headers(&headers)?;

    if payload.operations.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({ "code": 400, "message": "Transaction operations list cannot be empty." })),
        ));
    }

    let mut tx = state.db.begin().await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "code": 500, "message": format!("Failed to start transaction: {}", e) })),
        )
    })?;

    let mut results = Vec::with_capacity(payload.operations.len());
    let mut pending_realtime_events: Vec<RealtimeMessage> = Vec::new();

    for (idx, op) in payload.operations.iter().enumerate() {
        let step_num = idx + 1;
        let action_lower = op.action.trim().to_lowercase();

        match action_lower.as_str() {
            "create" | "insert" => {
                let col_name = op.collection.as_deref().unwrap_or("").trim();
                if col_name.is_empty() {
                    let _ = tx.rollback().await;
                    return Err((
                        StatusCode::BAD_REQUEST,
                        Json(json!({
                            "code": 400,
                            "message": format!("Step {}: 'collection' is required for create action.", step_num)
                        })),
                    ));
                }

                let col = get_collection_by_name_or_id(&state.db, col_name)
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
                            Json(json!({
                                "code": 404,
                                "message": format!("Step {}: Collection '{}' not found.", step_num, col_name)
                            })),
                        )
                    })?;

                check_permission(RecordAction::Create, &col, auth.as_ref(), None)?;

                let data = op.data.clone().unwrap_or_default();
                let record = match create_record_tx(&mut tx, &state.db, &col, data).await {
                    Ok(r) => r,
                    Err(e) => {
                        let _ = tx.rollback().await;
                        return Err((
                            StatusCode::BAD_REQUEST,
                            Json(json!({
                                "code": 400,
                                "message": format!("Step {} (create on '{}') failed: {}", step_num, col.name, e)
                            })),
                        ));
                    }
                };

                if col.realtime {
                    if let Ok(val) = serde_json::to_value(&record) {
                        pending_realtime_events.push(RealtimeMessage::new("create", &col.name, val));
                    }
                }

                results.push(json!({
                    "step": step_num,
                    "action": "create",
                    "collection": col.name,
                    "record": record
                }));
            }

            "update" | "patch" => {
                let col_name = op.collection.as_deref().unwrap_or("").trim();
                let rec_id = op.id.as_deref().unwrap_or("").trim();
                if col_name.is_empty() || rec_id.is_empty() {
                    let _ = tx.rollback().await;
                    return Err((
                        StatusCode::BAD_REQUEST,
                        Json(json!({
                            "code": 400,
                            "message": format!("Step {}: 'collection' and 'id' are required for update action.", step_num)
                        })),
                    ));
                }

                let col = get_collection_by_name_or_id(&state.db, col_name)
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
                            Json(json!({
                                "code": 404,
                                "message": format!("Step {}: Collection '{}' not found.", step_num, col_name)
                            })),
                        )
                    })?;

                let existing = get_record_tx(&mut tx, &state.db, &col, rec_id)
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
                            Json(json!({
                                "code": 404,
                                "message": format!("Step {}: Record '{}' not found in '{}'.", step_num, rec_id, col.name)
                            })),
                        )
                    })?;

                check_permission(
                    RecordAction::Update,
                    &col,
                    auth.as_ref(),
                    Some(&existing.data),
                )?;

                let data = op.data.clone().unwrap_or_default();
                let record = match update_record_tx(&mut tx, &state.db, &col, rec_id, data).await {
                    Ok(r) => r,
                    Err(e) => {
                        let _ = tx.rollback().await;
                        return Err((
                            StatusCode::BAD_REQUEST,
                            Json(json!({
                                "code": 400,
                                "message": format!("Step {} (update '{}' in '{}') failed: {}", step_num, rec_id, col.name, e)
                            })),
                        ));
                    }
                };

                if col.realtime {
                    if let Ok(val) = serde_json::to_value(&record) {
                        pending_realtime_events.push(RealtimeMessage::new("update", &col.name, val));
                    }
                }

                results.push(json!({
                    "step": step_num,
                    "action": "update",
                    "collection": col.name,
                    "record": record
                }));
            }

            "delete" | "remove" => {
                let col_name = op.collection.as_deref().unwrap_or("").trim();
                let rec_id = op.id.as_deref().unwrap_or("").trim();
                if col_name.is_empty() || rec_id.is_empty() {
                    let _ = tx.rollback().await;
                    return Err((
                        StatusCode::BAD_REQUEST,
                        Json(json!({
                            "code": 400,
                            "message": format!("Step {}: 'collection' and 'id' are required for delete action.", step_num)
                        })),
                    ));
                }

                let col = get_collection_by_name_or_id(&state.db, col_name)
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
                            Json(json!({
                                "code": 404,
                                "message": format!("Step {}: Collection '{}' not found.", step_num, col_name)
                            })),
                        )
                    })?;

                let existing = get_record_tx(&mut tx, &state.db, &col, rec_id)
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
                            Json(json!({
                                "code": 404,
                                "message": format!("Step {}: Record '{}' not found in '{}'.", step_num, rec_id, col.name)
                            })),
                        )
                    })?;

                check_permission(
                    RecordAction::Delete,
                    &col,
                    auth.as_ref(),
                    Some(&existing.data),
                )?;

                match delete_record_tx(&mut tx, &state.db, &col, rec_id).await {
                    Ok(true) => {}
                    Ok(false) => {
                        let _ = tx.rollback().await;
                        return Err((
                            StatusCode::NOT_FOUND,
                            Json(json!({
                                "code": 404,
                                "message": format!("Step {}: Record '{}' not found in '{}'.", step_num, rec_id, col.name)
                            })),
                        ));
                    }
                    Err(e) => {
                        let _ = tx.rollback().await;
                        return Err((
                            StatusCode::BAD_REQUEST,
                            Json(json!({
                                "code": 400,
                                "message": format!("Step {} (delete '{}' in '{}') failed: {}", step_num, rec_id, col.name, e)
                            })),
                        ));
                    }
                }

                if col.realtime {
                    pending_realtime_events.push(RealtimeMessage::new(
                        "delete",
                        &col.name,
                        json!({ "id": rec_id }),
                    ));
                }

                results.push(json!({
                    "step": step_num,
                    "action": "delete",
                    "collection": col.name,
                    "id": rec_id
                }));
            }

            "sql" | "query" | "execute" => {
                // Only admin / superadmin can execute raw SQL inside a transaction
                let is_admin = auth
                    .as_ref()
                    .map(|c| c.role == "admin" || c.role == "superadmin")
                    .unwrap_or(false);
                if !is_admin {
                    let _ = tx.rollback().await;
                    return Err((
                        StatusCode::FORBIDDEN,
                        Json(json!({
                            "code": 403,
                            "message": format!("Step {}: Raw SQL execution inside transactions requires administrator privileges.", step_num)
                        })),
                    ));
                }

                let raw_sql = op.sql.as_deref().unwrap_or("").trim();
                if raw_sql.is_empty() {
                    let _ = tx.rollback().await;
                    return Err((
                        StatusCode::BAD_REQUEST,
                        Json(json!({
                            "code": 400,
                            "message": format!("Step {}: 'sql' string is required for sql action.", step_num)
                        })),
                    ));
                }

                let bind_vals: Vec<Val> = op.params.iter().map(Val::from).collect();
                let is_select = raw_sql
                    .split_whitespace()
                    .next()
                    .map(|w| w.eq_ignore_ascii_case("SELECT") || w.eq_ignore_ascii_case("WITH"))
                    .unwrap_or(false);

                if is_select {
                    match tx.fetch_all(&state.db, raw_sql, &bind_vals).await {
                        Ok(rows) => {
                            results.push(json!({
                                "step": step_num,
                                "action": "sql",
                                "rows": rows
                            }));
                        }
                        Err(e) => {
                            let _ = tx.rollback().await;
                            return Err((
                                StatusCode::BAD_REQUEST,
                                Json(json!({
                                    "code": 400,
                                    "message": format!("Step {} SQL query failed: {}", step_num, e)
                                })),
                            ));
                        }
                    }
                } else {
                    match tx.execute(&state.db, raw_sql, &bind_vals).await {
                        Ok((rows_affected, last_insert_id)) => {
                            results.push(json!({
                                "step": step_num,
                                "action": "sql",
                                "rows_affected": rows_affected,
                                "last_insert_id": last_insert_id
                            }));
                        }
                        Err(e) => {
                            let _ = tx.rollback().await;
                            return Err((
                                StatusCode::BAD_REQUEST,
                                Json(json!({
                                    "code": 400,
                                    "message": format!("Step {} SQL execution failed: {}", step_num, e)
                                })),
                            ));
                        }
                    }
                }
            }

            other => {
                let _ = tx.rollback().await;
                return Err((
                    StatusCode::BAD_REQUEST,
                    Json(json!({
                        "code": 400,
                        "message": format!("Step {}: Unsupported transaction action '{}'. Use 'create', 'update', 'delete', or 'sql'.", step_num, other)
                    })),
                ));
            }
        }
    }

    // Commit the transaction atomically
    tx.commit().await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "code": 500, "message": format!("Failed to commit transaction: {}", e) })),
        )
    })?;

    // Publish all realtime events after successful commit
    for ev in pending_realtime_events {
        state.realtime.broadcast(ev);
    }

    Ok(Json(json!({
        "committed": true,
        "total_operations": results.len(),
        "results": results
    })))
}

/// Execute parameterized custom SQL query / statement (Admin only).
///
/// Endpoint: `POST /api/sql` or `POST /api/sql/query`
pub async fn execute_custom_sql(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<RawSqlRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<Value>)> {
    let auth = authenticate_headers(&headers)?;
    let is_admin = auth
        .as_ref()
        .map(|c| c.role == "admin" || c.role == "superadmin")
        .unwrap_or(false);

    if !is_admin {
        return Err((
            StatusCode::FORBIDDEN,
            Json(json!({
                "code": 403,
                "message": "Only administrators can execute raw SQL queries."
            })),
        ));
    }

    let raw_sql = payload.sql.trim();
    if raw_sql.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({ "code": 400, "message": "SQL query cannot be empty." })),
        ));
    }

    let bind_vals: Vec<Val> = payload.params.iter().map(Val::from).collect();
    let is_select = raw_sql
        .split_whitespace()
        .next()
        .map(|w| {
            w.eq_ignore_ascii_case("SELECT")
                || w.eq_ignore_ascii_case("WITH")
                || w.eq_ignore_ascii_case("PRAGMA")
                || w.eq_ignore_ascii_case("SHOW")
                || w.eq_ignore_ascii_case("EXPLAIN")
        })
        .unwrap_or(false);

    if payload.transaction {
        let mut tx = state.db.begin().await.map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "code": 500, "message": e.to_string() })),
            )
        })?;

        if is_select {
            let rows = tx.fetch_all(&state.db, raw_sql, &bind_vals).await.map_err(|e| {
                (
                    StatusCode::BAD_REQUEST,
                    Json(json!({ "code": 400, "message": e.to_string() })),
                )
            })?;
            tx.commit().await.map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({ "code": 500, "message": e.to_string() })),
                )
            })?;
            return Ok(Json(json!({
                "items": rows,
                "totalItems": rows.len()
            })));
        } else {
            let (rows_affected, last_insert_id) =
                tx.execute(&state.db, raw_sql, &bind_vals).await.map_err(|e| {
                    (
                        StatusCode::BAD_REQUEST,
                        Json(json!({ "code": 400, "message": e.to_string() })),
                    )
                })?;
            tx.commit().await.map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({ "code": 500, "message": e.to_string() })),
                )
            })?;
            return Ok(Json(json!({
                "rows_affected": rows_affected,
                "last_insert_id": last_insert_id
            })));
        }
    }

    if is_select {
        let rows = state
            .db
            .fetch_all(raw_sql, &bind_vals)
            .await
            .map_err(|e| {
                (
                    StatusCode::BAD_REQUEST,
                    Json(json!({ "code": 400, "message": e.to_string() })),
                )
            })?;
        Ok(Json(json!({
            "items": rows,
            "totalItems": rows.len()
        })))
    } else {
        let (rows_affected, last_insert_id) = state
            .db
            .execute(raw_sql, &bind_vals)
            .await
            .map_err(|e| {
                (
                    StatusCode::BAD_REQUEST,
                    Json(json!({ "code": 400, "message": e.to_string() })),
                )
            })?;
        Ok(Json(json!({
            "rows_affected": rows_affected,
            "last_insert_id": last_insert_id
        })))
    }
}
