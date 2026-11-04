use axum::{
    body::Body,
    extract::{Multipart, Path, State},
    http::{header, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use chrono::Utc;
use serde::Serialize;
use serde_json::json;
use std::path::{Path as StdPath, PathBuf};
use tokio::fs::File;
use tokio_util::io::ReaderStream;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

use crate::core::app::AppState;
use crate::core::db::list_collections;

#[derive(Serialize)]
pub struct BackupInfo {
    pub key: String,
    pub size: u64,
    pub created: String,
    pub is_zip: bool,
    pub has_storage: bool,
    pub storage_files: usize,
}

fn get_backups_dir(data_dir: &StdPath) -> PathBuf {
    data_dir.join("backups")
}

/// Recursively add all files from a directory into a zip archive with relative paths
fn add_dir_to_zip<W: std::io::Write + std::io::Seek>(
    zip: &mut ZipWriter<W>,
    base_dir: &StdPath,
    current_dir: &StdPath,
    options: SimpleFileOptions,
) -> Result<(usize, u64), std::io::Error> {
    let mut files_count = 0;
    let mut total_bytes = 0;

    if !current_dir.exists() {
        return Ok((0, 0));
    }

    let entries = match std::fs::read_dir(current_dir) {
        Ok(e) => e,
        Err(_) => return Ok((0, 0)),
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            let (sub_count, sub_bytes) = add_dir_to_zip(zip, base_dir, &path, options)?;
            files_count += sub_count;
            total_bytes += sub_bytes;
        } else if path.is_file() {
            if let Ok(rel_path) = path.strip_prefix(base_dir) {
                let rel_str = rel_path.to_string_lossy().replace('\\', "/");
                let zip_entry_name = format!("storage/{}", rel_str);

                if let Ok(mut f) = std::fs::File::open(&path) {
                    if zip.start_file(&zip_entry_name, options).is_ok() {
                        if let Ok(copied) = std::io::copy(&mut f, zip) {
                            files_count += 1;
                            total_bytes += copied;
                        }
                    }
                }
            }
        }
    }

    Ok((files_count, total_bytes))
}

/// Inspect a backup file to check if it has storage files and get metadata
fn inspect_backup_file(path: &StdPath) -> (bool, bool, usize) {
    let file_name = path.file_name().and_then(|f| f.to_str()).unwrap_or("");
    if !file_name.ends_with(".zip") {
        return (false, false, 0);
    }

    let file = match std::fs::File::open(path) {
        Ok(f) => f,
        Err(_) => return (true, false, 0),
    };

    let mut archive = match ZipArchive::new(file) {
        Ok(a) => a,
        Err(_) => return (true, false, 0),
    };

    // Check manifest if available
    if let Ok(mut manifest_file) = archive.by_name("manifest.json") {
        let mut content = String::new();
        if std::io::Read::read_to_string(&mut manifest_file, &mut content).is_ok() {
            if let Ok(manifest) = serde_json::from_str::<serde_json::Value>(&content) {
                let files_count = manifest.get("storage_files").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
                return (true, files_count > 0, files_count);
            }
        }
    }

    // Fallback: scan archive entries
    let mut storage_files = 0;
    for i in 0..archive.len() {
        if let Ok(entry) = archive.by_index(i) {
            if entry.name().starts_with("storage/") && !entry.is_dir() {
                storage_files += 1;
            }
        }
    }

    (true, storage_files > 0, storage_files)
}

/// List all available backups with metadata
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

                let (is_zip, has_storage, storage_files) = inspect_backup_file(&path);

                backups.push(BackupInfo {
                    key: file_name,
                    size: metadata.len(),
                    created: modified,
                    is_zip,
                    has_storage,
                    storage_files,
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

/// Create a unified point-in-time backup bundle (.zip containing database snapshot + physical storage uploads + manifest)
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
    let zip_filename = format!("padi_backup_{}.zip", timestamp);
    let backup_path = backup_dir.join(&zip_filename);

    // 1. Prepare database snapshot cleanly
    let temp_db_path = backup_dir.join(format!("temp_snapshot_{}.db", timestamp));
    let temp_json_path = backup_dir.join(format!("temp_snapshot_{}.json", timestamp));

    let is_sqlite = state.db.is_sqlite();
    if is_sqlite {
        // SQLite WAL Checkpoint to flush all memory pages to disk
        let _ = state
            .db
            .execute("PRAGMA wal_checkpoint(TRUNCATE);", &[])
            .await;

        // Try SQLite VACUUM INTO for a safe, online, atomic, compacted snapshot
        let vacuum_sql = format!(
            "VACUUM INTO '{}';",
            temp_db_path.to_string_lossy().replace('\\', "/")
        );
        let res = state.db.execute(&vacuum_sql, &[]).await;

        if res.is_err() {
            // Fallback: Copy data.db directly if VACUUM INTO failed
            let db_path = state.data_dir.join("data.db");
            tokio::fs::copy(&db_path, &temp_db_path)
                .await
                .map_err(|copy_err| {
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(json!({ "code": 500, "message": format!("Database snapshot failed: {}", copy_err) })),
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
        tokio::fs::write(&temp_json_path, bytes).await.map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "code": 500, "message": format!("Database dump failed: {}", e) })),
            )
        })?;
    }

    // 2. Package database snapshot + physical storage/ files into a compressed .zip archive
    let storage_dir = state.storage_dir.clone();
    let b_path = backup_path.clone();
    let t_db = temp_db_path.clone();
    let t_json = temp_json_path.clone();
    let driver_name = state.db.driver_name().to_string();

    let zip_result = tokio::task::spawn_blocking(move || -> Result<(usize, u64), String> {
        let zip_file = std::fs::File::create(&b_path)
            .map_err(|e| format!("Failed to create backup zip file: {}", e))?;
        let mut zip = ZipWriter::new(zip_file);
        let options = SimpleFileOptions::default()
            .compression_method(CompressionMethod::Deflated);

        // A. Add database snapshot
        if is_sqlite && t_db.exists() {
            if let Ok(mut f) = std::fs::File::open(&t_db) {
                zip.start_file("data.db", options)
                    .map_err(|e| format!("Failed to add data.db to zip: {}", e))?;
                std::io::copy(&mut f, &mut zip)
                    .map_err(|e| format!("Failed to write data.db stream to zip: {}", e))?;
            }
        } else if !is_sqlite && t_json.exists() {
            if let Ok(mut f) = std::fs::File::open(&t_json) {
                zip.start_file("database.json", options)
                    .map_err(|e| format!("Failed to add database.json to zip: {}", e))?;
                std::io::copy(&mut f, &mut zip)
                    .map_err(|e| format!("Failed to write database.json stream to zip: {}", e))?;
            }
        }

        // B. Add physical upload storage files from storage_dir
        let (files_count, total_bytes) = add_dir_to_zip(&mut zip, &storage_dir, &storage_dir, options)
            .map_err(|e| format!("Failed to bundle storage files: {}", e))?;

        // C. Add manifest.json with full metadata
        let manifest = json!({
            "app": "PadiBase",
            "version": "1.0",
            "created_at": Utc::now().to_rfc3339(),
            "driver": driver_name,
            "db_type": if is_sqlite { "sqlite" } else { "json_snapshot" },
            "db_file": if is_sqlite { "data.db" } else { "database.json" },
            "has_storage": files_count > 0,
            "storage_files": files_count,
            "storage_bytes": total_bytes
        });
        let manifest_bytes = serde_json::to_vec_pretty(&manifest).unwrap_or_default();
        zip.start_file("manifest.json", options)
            .map_err(|e| format!("Failed to add manifest.json to zip: {}", e))?;
        std::io::Write::write_all(&mut zip, &manifest_bytes)
            .map_err(|e| format!("Failed to write manifest.json: {}", e))?;

        zip.finish()
            .map_err(|e| format!("Failed to finalize zip archive: {}", e))?;

        // Clean up temporary snapshot files
        let _ = std::fs::remove_file(&t_db);
        let _ = std::fs::remove_file(&t_json);

        Ok((files_count, total_bytes))
    })
    .await
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "code": 500, "message": format!("Zip worker crashed: {}", e) })),
        )
    })?;

    let (storage_files, _storage_bytes) = zip_result.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "code": 500, "message": e })),
        )
    })?;

    let metadata = tokio::fs::metadata(&backup_path).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "code": 500, "message": e.to_string() })),
        )
    })?;

    Ok((
        StatusCode::CREATED,
        Json(BackupInfo {
            key: zip_filename,
            size: metadata.len(),
            created: Utc::now().to_rfc3339(),
            is_zip: true,
            has_storage: storage_files > 0,
            storage_files,
        }),
    ))
}

/// Upload an existing backup archive (.zip, .db, or .json) from local machine
pub async fn upload_backup(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    let backup_dir = get_backups_dir(&state.data_dir);
    if !backup_dir.exists() {
        let _ = tokio::fs::create_dir_all(&backup_dir).await;
    }

    let mut saved_filename = None;
    let mut saved_size = 0u64;

    while let Ok(Some(field)) = multipart.next_field().await {
        let raw_filename = field.file_name().map(String::from).unwrap_or_default();
        if raw_filename.trim().is_empty() {
            continue;
        }

        // Clean & sanitize filename
        let clean_name = StdPath::new(&raw_filename)
            .file_name()
            .and_then(|f| f.to_str())
            .unwrap_or("backup.zip")
            .to_string();

        let safe_name: String = clean_name
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || c == '.' || c == '-' || c == '_' {
                    c
                } else {
                    '_'
                }
            })
            .collect();

        if !safe_name.ends_with(".zip") && !safe_name.ends_with(".db") && !safe_name.ends_with(".json") {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(json!({ "code": 400, "message": "Only .zip, .db, or .json backup files can be uploaded." })),
            ));
        }

        let target_path = backup_dir.join(&safe_name);
        let data = field.bytes().await.map_err(|e| {
            (
                StatusCode::BAD_REQUEST,
                Json(json!({ "code": 400, "message": format!("Failed to read backup payload: {}", e) })),
            )
        })?;

        saved_size = data.len() as u64;
        tokio::fs::write(&target_path, &data).await.map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "code": 500, "message": format!("Failed to save backup: {}", e) })),
            )
        })?;

        saved_filename = Some(safe_name);
        break;
    }

    let filename = saved_filename.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(json!({ "code": 400, "message": "No backup file uploaded." })),
        )
    })?;

    let backup_path = backup_dir.join(&filename);
    let (is_zip, has_storage, storage_files) = inspect_backup_file(&backup_path);

    Ok((
        StatusCode::CREATED,
        Json(BackupInfo {
            key: filename,
            size: saved_size,
            created: Utc::now().to_rfc3339(),
            is_zip,
            has_storage,
            storage_files,
        }),
    ))
}

/// Download a backup file with appropriate MIME headers
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
    let content_type = if filename.ends_with(".zip") {
        "application/zip"
    } else if filename.ends_with(".json") {
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

/// Delete a backup file
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

/// Restore database and physical storage files from a unified .zip (or legacy .db / .json) backup archive
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

    // 1. If Unified .ZIP Backup: Extract data.db + physical storage/ files
    if filename.ends_with(".zip") {
        let b_path = backup_path.clone();
        let d_dir = state.data_dir.clone();
        let s_dir = state.storage_dir.clone();
        let is_sqlite = state.db.is_sqlite();

        let extract_result = tokio::task::spawn_blocking(move || -> Result<(bool, usize), String> {
            let file = std::fs::File::open(&b_path)
                .map_err(|e| format!("Failed to open backup zip: {}", e))?;
            let mut archive = ZipArchive::new(file)
                .map_err(|e| format!("Failed to read backup zip archive: {}", e))?;

            let mut restored_db = false;
            let mut files_restored = 0;

            for i in 0..archive.len() {
                let mut file = archive
                    .by_index(i)
                    .map_err(|e| format!("Error reading zip entry #{}: {}", i, e))?;
                let entry_name = file.name().to_string();

                // A. Database file extraction
                if entry_name == "data.db" && is_sqlite {
                    let temp_db_path = d_dir.join("data.db.restoring");
                    let mut out = std::fs::File::create(&temp_db_path)
                        .map_err(|e| format!("Failed to create temporary restore DB: {}", e))?;
                    std::io::copy(&mut file, &mut out)
                        .map_err(|e| format!("Failed to extract data.db: {}", e))?;
                    drop(out);

                    // Overwrite data.db
                    let target_db = d_dir.join("data.db");
                    std::fs::rename(&temp_db_path, &target_db)
                        .or_else(|_| std::fs::copy(&temp_db_path, &target_db).map(|_| ()))
                        .map_err(|e| format!("Failed to replace data.db: {}", e))?;
                    let _ = std::fs::remove_file(&temp_db_path);

                    // Crucial SQLite Safety: remove stale WAL and SHM files
                    let _ = std::fs::remove_file(d_dir.join("data.db-wal"));
                    let _ = std::fs::remove_file(d_dir.join("data.db-shm"));
                    restored_db = true;
                }
                // B. Storage files extraction
                else if entry_name.starts_with("storage/") {
                    // Guard against Zip Slip directory traversal
                    if let Some(safe_path) = file.enclosed_name() {
                        if let Ok(rel_storage) = safe_path.strip_prefix("storage") {
                            let dest_path = s_dir.join(rel_storage);
                            if file.is_dir() {
                                let _ = std::fs::create_dir_all(&dest_path);
                            } else {
                                if let Some(parent) = dest_path.parent() {
                                    let _ = std::fs::create_dir_all(parent);
                                }
                                if let Ok(mut out) = std::fs::File::create(&dest_path) {
                                    if std::io::copy(&mut file, &mut out).is_ok() {
                                        files_restored += 1;
                                    }
                                }
                            }
                        }
                    }
                }
            }

            Ok((restored_db, files_restored))
        })
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "code": 500, "message": format!("Restore worker crashed: {}", e) })),
            )
        })?;

        let (restored_db, files_restored) = extract_result.map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "code": 500, "message": e })),
            )
        })?;

        // Invalidate all in-memory cache to ensure fresh introspection
        state.db.clear_all_schema_cache();

        if state.db.is_sqlite() {
            let _ = state
                .db
                .execute("PRAGMA wal_checkpoint(TRUNCATE);", &[])
                .await;
        }

        return Ok(Json(json!({
            "success": true,
            "message": format!(
                "Unified restore completed successfully from {}: database restored = {}, {} storage files extracted.",
                filename, restored_db, files_restored
            ),
            "database_restored": restored_db,
            "storage_files_restored": files_restored
        })));
    }

    // 2. Legacy SQLite .db single file restore
    if state.db.is_sqlite() && filename.ends_with(".db") {
        let db_path = state.data_dir.join("data.db");
        tokio::fs::copy(&backup_path, &db_path).await.map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "code": 500, "message": format!("Restore failed: {}", e) })),
            )
        })?;

        // Remove stale WAL and SHM files
        let _ = tokio::fs::remove_file(state.data_dir.join("data.db-wal")).await;
        let _ = tokio::fs::remove_file(state.data_dir.join("data.db-shm")).await;

        state.db.clear_all_schema_cache();
        let _ = state
            .db
            .execute("PRAGMA wal_checkpoint(TRUNCATE);", &[])
            .await;
    }

    Ok(Json(json!({
        "success": true,
        "message": format!("Database restored successfully from backup {}", filename)
    })))
}
