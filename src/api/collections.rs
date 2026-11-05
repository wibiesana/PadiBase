use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    Json,
};
use serde_json::json;

use crate::core::app::AppState;
use crate::core::auth::require_admin;
use crate::core::db::{
    create_or_update_collection_table, delete_collection, get_collection_by_name_or_id,
    is_safe_identifier, list_collections, rename_collection, save_collection_metadata,
};
use crate::core::schema::{generate_id, Collection};

type ApiError = (StatusCode, Json<serde_json::Value>);

fn bad_request(msg: impl Into<String>) -> ApiError {
    (
        StatusCode::BAD_REQUEST,
        Json(json!({ "code": 400, "message": msg.into() })),
    )
}

fn internal(context: &str, e: impl std::fmt::Display) -> ApiError {
    tracing::error!("{}: {}", context, e);
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(json!({ "code": 500, "message": format!("{}.", context) })),
    )
}

/// Validate a collection name and every schema field name before they are ever
/// interpolated into DDL statements.
fn validate_collection(col: &Collection) -> Result<(), ApiError> {
    let name = col.name.as_str();
    if name.is_empty() {
        return Err(bad_request("Collection name is required."));
    }
    if name.starts_with('_') {
        return Err(bad_request(
            "Collection name cannot start with underscore '_' (reserved system tables).",
        ));
    }
    if name.len() > 64
        || !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
        || !is_safe_identifier(name)
    {
        return Err(bad_request(
            "Collection name must only contain alphanumeric characters and underscores (e.g. 'posts', 'blog_articles').",
        ));
    }
    for field in &col.schema {
        if field.name.len() > 64 || !is_safe_identifier(&field.name) {
            return Err(bad_request(format!(
                "Invalid field name '{}'. Use only letters, digits and underscores.",
                field.name
            )));
        }
    }
    Ok(())
}

pub async fn list(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, ApiError> {
    require_admin(&state, &headers).await?;
    let cols = list_collections(&state.db)
        .await
        .map_err(|e| internal("Failed to list collections", e))?;

    Ok(Json(json!({
        "page": 1,
        "perPage": cols.len(),
        "totalItems": cols.len(),
        "totalPages": 1,
        "items": cols
    })))
}

pub async fn get_one(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(name_or_id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    require_admin(&state, &headers).await?;
    let col = get_collection_by_name_or_id(&state.db, &name_or_id)
        .await
        .map_err(|e| internal("Failed to load collection", e))?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(json!({ "code": 404, "message": "Collection not found." })),
            )
        })?;

    Ok(Json(col))
}

pub async fn create(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(mut payload): Json<Collection>,
) -> Result<impl IntoResponse, ApiError> {
    require_admin(&state, &headers).await?;
    payload.name = payload.name.trim().to_lowercase();
    validate_collection(&payload)?;

    // Periksa apakah nama sudah ada
    if let Ok(Some(_)) = get_collection_by_name_or_id(&state.db, &payload.name).await {
        return Err(bad_request("Collection name already exists."));
    }

    if payload.id.is_empty() {
        payload.id = generate_id();
    }
    let now = chrono::Utc::now().to_rfc3339();
    payload.created = now.clone();
    payload.updated = now;

    // 1. Buat tabel fisik
    create_or_update_collection_table(&state.db, &payload)
        .await
        .map_err(|e| internal("Failed to create table", e))?;

    // 2. Simpan metadata ke _collections
    save_collection_metadata(&state.db, &payload)
        .await
        .map_err(|e| internal("Failed to save metadata", e))?;

    Ok((StatusCode::CREATED, Json(payload)))
}

pub async fn update(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(name_or_id): Path<String>,
    Json(mut payload): Json<Collection>,
) -> Result<impl IntoResponse, ApiError> {
    require_admin(&state, &headers).await?;
    let existing = get_collection_by_name_or_id(&state.db, &name_or_id)
        .await
        .map_err(|e| internal("Failed to load collection", e))?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(json!({ "code": 404, "message": "Collection not found." })),
            )
        })?;

    // The physical table name is immutable here; never trust the name sent by the client.
    payload.id = existing.id;
    payload.name = existing.name;
    payload.created = existing.created;
    payload.updated = chrono::Utc::now().to_rfc3339();
    validate_collection(&payload)?;

    // Sinkronisasi skema tabel (tambah kolom baru jika ada)
    create_or_update_collection_table(&state.db, &payload)
        .await
        .map_err(|e| internal("Failed to update table", e))?;

    save_collection_metadata(&state.db, &payload)
        .await
        .map_err(|e| internal("Failed to save metadata", e))?;

    Ok(Json(payload))
}

#[derive(serde::Deserialize)]
pub struct RenamePayload {
    pub name: String,
}

pub async fn rename(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(name_or_id): Path<String>,
    Json(payload): Json<RenamePayload>,
) -> Result<impl IntoResponse, ApiError> {
    require_admin(&state, &headers).await?;
    let existing = get_collection_by_name_or_id(&state.db, &name_or_id)
        .await
        .map_err(|e| internal("Failed to load collection", e))?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(json!({ "code": 404, "message": "Collection not found." })),
            )
        })?;
    if existing.name.starts_with('_') || existing.name == "users" {
        return Err(bad_request("System tables cannot be renamed."));
    }

    let mut probe = existing.clone();
    probe.name = payload.name.trim().to_lowercase();
    validate_collection(&probe)?;
    if probe.name == existing.name {
        return Ok(Json(existing));
    }
    if let Ok(Some(_)) = get_collection_by_name_or_id(&state.db, &probe.name).await {
        return Err(bad_request("Collection name already exists."));
    }

    let renamed = rename_collection(&state.db, &existing.id, &probe.name)
        .await
        .map_err(|e| internal("Failed to rename table", e))?;
    Ok(Json(renamed))
}

pub async fn delete(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(name_or_id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    require_admin(&state, &headers).await?;
    if name_or_id.starts_with('_') {
        return Err(bad_request("System tables cannot be deleted."));
    }
    let deleted = delete_collection(&state.db, &name_or_id)
        .await
        .map_err(|e| internal("Failed to delete collection", e))?;

    if deleted {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err((
            StatusCode::NOT_FOUND,
            Json(json!({ "code": 404, "message": "Collection not found." })),
        ))
    }
}

#[derive(serde::Deserialize)]
pub struct ImportCollectionsPayload {
    pub collections: Vec<Collection>,
}

pub async fn import_collections(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<ImportCollectionsPayload>,
) -> Result<impl IntoResponse, ApiError> {
    require_admin(&state, &headers).await?;
    let now = chrono::Utc::now().to_rfc3339();

    // Validate everything first so a bad entry cannot leave a half-applied import.
    let mut prepared = Vec::with_capacity(payload.collections.len());
    for mut col in payload.collections {
        col.name = col.name.trim().to_lowercase();
        if col.name.is_empty() || col.name.starts_with('_') {
            continue;
        }
        validate_collection(&col)?;
        prepared.push(col);
    }

    for mut col in prepared {
        if let Ok(Some(existing)) = get_collection_by_name_or_id(&state.db, &col.name).await {
            col.id = existing.id;
            col.created = existing.created;
        } else if col.id.is_empty() {
            col.id = generate_id();
            col.created = now.clone();
        }
        col.updated = now.clone();
        create_or_update_collection_table(&state.db, &col)
            .await
            .map_err(|e| internal("Failed to import collection", e))?;
        save_collection_metadata(&state.db, &col)
            .await
            .map_err(|e| internal("Failed to import collection metadata", e))?;
    }
    Ok(StatusCode::NO_CONTENT)
}

pub async fn sync(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, ApiError> {
    require_admin(&state, &headers).await?;
    let synced = crate::core::db::sync_existing_tables(&state.db)
        .await
        .map_err(|e| internal("Failed to introspect tables", e))?;

    Ok(Json(json!({
        "success": true,
        "message": format!("Successfully synchronized {} table(s)", synced.len()),
        "synced": synced,
        "count": synced.len()
    })))
}
