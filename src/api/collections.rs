use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde_json::json;

use crate::core::app::AppState;
use crate::core::db::{
    create_or_update_collection_table, delete_collection, get_collection_by_name_or_id,
    list_collections, save_collection_metadata,
};
use crate::core::schema::{generate_id, Collection};

pub async fn list(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    let cols = list_collections(&state.db).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "code": 500, "message": e.to_string() })),
        )
    })?;

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
    Path(name_or_id): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    let col = get_collection_by_name_or_id(&state.db, &name_or_id)
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

    Ok(Json(col))
}

pub async fn create(
    State(state): State<AppState>,
    Json(mut payload): Json<Collection>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    let name = payload.name.trim().to_lowercase();
    if name.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({ "code": 400, "message": "Collection name is required." })),
        ));
    }

    if name.starts_with('_') {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({ "code": 400, "message": "Collection name cannot start with underscore '_' (reserved system tables)." })),
        ));
    }

    if !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({ "code": 400, "message": "Collection name must only contain alphanumeric characters and underscores (e.g. 'posts', 'blog_articles')." })),
        ));
    }

    payload.name = name;

    // Periksa apakah nama sudah ada
    if let Ok(Some(_)) = get_collection_by_name_or_id(&state.db, &payload.name).await {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({ "code": 400, "message": "Collection name already exists." })),
        ));
    }

    if payload.id.is_empty() {
        payload.id = generate_id();
    }
    let now = chrono::Utc::now().to_rfc3339();
    payload.created = now.clone();
    payload.updated = now;

    // 1. Buat tabel fisik di SQLite
    create_or_update_collection_table(&state.db, &payload)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "code": 500, "message": format!("Failed to create table: {}", e) })),
            )
        })?;

    // 2. Simpan metadata ke _collections
    save_collection_metadata(&state.db, &payload)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "code": 500, "message": format!("Failed to save metadata: {}", e) })),
            )
        })?;

    Ok((StatusCode::CREATED, Json(payload)))
}

pub async fn update(
    State(state): State<AppState>,
    Path(name_or_id): Path<String>,
    Json(mut payload): Json<Collection>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    let existing = get_collection_by_name_or_id(&state.db, &name_or_id)
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

    payload.id = existing.id;
    payload.created = existing.created;
    payload.updated = chrono::Utc::now().to_rfc3339();

    // Sinkronisasi skema tabel SQLite (tambah kolom baru jika ada)
    create_or_update_collection_table(&state.db, &payload)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "code": 500, "message": format!("Failed to update table: {}", e) })),
            )
        })?;

    save_collection_metadata(&state.db, &payload)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "code": 500, "message": format!("Failed to save metadata: {}", e) })),
            )
        })?;

    Ok(Json(payload))
}

pub async fn delete(
    State(state): State<AppState>,
    Path(name_or_id): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    let deleted = delete_collection(&state.db, &name_or_id).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "code": 500, "message": e.to_string() })),
        )
    })?;

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
    Json(payload): Json<ImportCollectionsPayload>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    let now = chrono::Utc::now().to_rfc3339();
    for mut col in payload.collections {
        let name = col.name.trim().to_lowercase();
        if name.is_empty() || name.starts_with('_') {
            continue;
        }
        col.name = name;
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
            .map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({ "code": 500, "message": e.to_string() })),
                )
            })?;
        save_collection_metadata(&state.db, &col)
            .await
            .map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({ "code": 500, "message": e.to_string() })),
                )
            })?;
    }
    Ok(StatusCode::NO_CONTENT)
}

pub async fn sync(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    let synced = crate::core::db::sync_existing_tables(&state.db)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "code": 500, "message": format!("Failed to introspect tables: {}", e) })),
            )
        })?;

    Ok(Json(json!({
        "success": true,
        "message": format!("Successfully synchronized {} table(s)", synced.len()),
        "synced": synced,
        "count": synced.len()
    })))
}
