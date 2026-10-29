use axum::{
    body::Body,
    extract::{Path, State},
    http::{header, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use chrono::Utc;
use serde::Serialize;
use serde_json::json;
use std::path::PathBuf;
use tokio::fs::File;
use tokio_util::io::ReaderStream;

use crate::core::app::AppState;
use crate::core::db::list_collections;

#[derive(Serialize)]
pub struct BackupInfo {
    pub key: String,
    pub size: u64,
    pub created: String,
}

fn get_backups_dir(data_dir: &std::path::Path) -> PathBuf {
    data_dir.join("backups")
}

pub async fn list_backups(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    let backup_dir = get_backups_dir(&state.data_dir);
    if !backup_dir.exists() {
        let _ = tokio::fs::create_dir_all(&backup_dir).await;
    }

    let mut backups = Vec::new();
    let mut dir = tokio::fs::read_dir(&backup_dir).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "code": 500, "message": e.to_string() })),
        )
    })?;

    while let Ok(Some(entry)) = dir.next_entry().await {
        let path = entry.path();
        if path.is_file() {
            if let Ok(metadata) = entry.metadata().await {
                let file_name = entry.file_name().to_string_lossy().to_string();
                let modified = metadata
                    .modified()
                    .ok()
                    .and_then(|t| chrono::DateTime::<Utc>::from(t).to_rfc3339().into())
                    .unwrap_or_else(|| Utc::now().to_rfc3339());

                backups.push(BackupInfo {
                    key: file_name,
                    size: metadata.len(),
                    created: modified,
                });
            }
        }
    }

    backups.sort_by(|a, b| b.created.cmp(&a.created));

    Ok(Json(json!({
        "items": backups,
        "totalItems": backups.len()
    })))
}

pub async fn create_backup(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    let backup_dir = get_backups_dir(&state.data_dir);
    if !backup_dir.exists() {
        tokio::fs::create_dir_all(&backup_dir).await.map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "code": 500, "message": e.to_string() })),
            )
        })?;
    }

    let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
    let filename = if state.db.is_sqlite() {
        format!("padi_backup_{}.db", timestamp)
    } else {
        format!("padi_backup_{}_{}.json", state.db.driver_name(), timestamp)
    };
    let backup_path = backup_dir.join(&filename);

    if state.db.is_sqlite() {
        // Checkpoint SQLite WAL to make sure all data is flushed to disk
        let _ = state
            .db
            .execute("PRAGMA wal_checkpoint(TRUNCATE);", &[])
            .await;

        // Use SQLite VACUUM INTO to create an atomic, safe online backup
        let vacuum_sql = format!(
            "VACUUM INTO '{}';",
            backup_path.to_string_lossy().replace('\\', "/")
        );
        let res = state.db.execute(&vacuum_sql, &[]).await;

        if let Err(e) = res {
            // Fallback: Copy data.db directly if VACUUM INTO is not supported or failed
            let db_path = state.data_dir.join("data.db");
            tokio::fs::copy(&db_path, &backup_path)
                .await
                .map_err(|copy_err| {
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(json!({ "code": 500, "message": format!("Backup failed: {} / {}", e, copy_err) })),
                    )
                })?;
        }
    } else {
        // Logical JSON snapshot for MySQL / MariaDB / PostgreSQL
        let collections = list_collections(&state.db).await.unwrap_or_default();
        let mut records_by_col = serde_json::Map::new();
        for col in &collections {
            let rows = state
                .db
                .fetch_all(&format!("SELECT * FROM `{}`", col.name), &[])
                .await
                .unwrap_or_default();
            records_by_col.insert(col.name.clone(), json!(rows));
        }
        let snapshot = json!({
            "driver": state.db.driver_name(),
            "created": Utc::now().to_rfc3339(),
            "collections": collections,
            "data": records_by_col,
        });
        let bytes = serde_json::to_vec_pretty(&snapshot).unwrap_or_default();
        tokio::fs::write(&backup_path, bytes).await.map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "code": 500, "message": format!("Backup failed: {}", e) })),
            )
        })?;
    }

    let metadata = tokio::fs::metadata(&backup_path).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "code": 500, "message": e.to_string() })),
        )
    })?;

    Ok((
        StatusCode::CREATED,
        Json(BackupInfo {
            key: filename,
            size: metadata.len(),
            created: Utc::now().to_rfc3339(),
        }),
    ))
}

pub async fn download_backup(
    State(state): State<AppState>,
    Path(filename): Path<String>,
) -> Result<Response, (StatusCode, Json<serde_json::Value>)> {
    if filename.contains("..") || filename.contains('/') || filename.contains('\\') {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({ "code": 400, "message": "Invalid filename." })),
        ));
    }

    let backup_path = get_backups_dir(&state.data_dir).join(&filename);
    if !backup_path.exists() {
        return Err((
            StatusCode::NOT_FOUND,
            Json(json!({ "code": 404, "message": "Backup file not found." })),
        ));
    }

    let file = File::open(&backup_path).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "code": 500, "message": e.to_string() })),
        )
    })?;

    let stream = ReaderStream::new(file);
    let body = Body::from_stream(stream);
    let disposition = format!("attachment; filename=\"{}\"", filename);
    let content_type = if filename.ends_with(".json") {
        "application/json"
    } else {
        "application/x-sqlite3"
    };

    let res = Response::builder()
        .header(header::CONTENT_TYPE, HeaderValue::from_static(content_type))
        .header(
            header::CONTENT_DISPOSITION,
            HeaderValue::from_str(&disposition).unwrap_or(HeaderValue::from_static("attachment")),
        )
        .body(body)
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "code": 500, "message": e.to_string() })),
            )
        })?;

    Ok(res)
}

pub async fn delete_backup(
    State(state): State<AppState>,
    Path(filename): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    if filename.contains("..") || filename.contains('/') || filename.contains('\\') {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({ "code": 400, "message": "Invalid filename." })),
        ));
    }

    let backup_path = get_backups_dir(&state.data_dir).join(&filename);
    if !backup_path.exists() {
        return Err((
            StatusCode::NOT_FOUND,
            Json(json!({ "code": 404, "message": "Backup file not found." })),
        ));
    }

    tokio::fs::remove_file(&backup_path).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "code": 500, "message": e.to_string() })),
        )
    })?;

    Ok(StatusCode::NO_CONTENT)
}

pub async fn restore_backup(
    State(state): State<AppState>,
    Path(filename): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    if filename.contains("..") || filename.contains('/') || filename.contains('\\') {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({ "code": 400, "message": "Invalid filename." })),
        ));
    }

    let backup_path = get_backups_dir(&state.data_dir).join(&filename);
    if !backup_path.exists() {
        return Err((
            StatusCode::NOT_FOUND,
            Json(json!({ "code": 404, "message": "Backup file not found." })),
        ));
    }

    if state.db.is_sqlite() && filename.ends_with(".db") {
        let db_path = state.data_dir.join("data.db");
        tokio::fs::copy(&backup_path, &db_path).await.map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "code": 500, "message": format!("Restore failed: {}", e) })),
            )
        })?;
    }

    Ok(Json(json!({
        "success": true,
        "message": format!("Database restored successfully from backup {}", filename)
    })))
}
