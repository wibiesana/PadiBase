use axum::{
    body::Body,
    extract::{Multipart, Path, State},
    http::{header, HeaderName, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use mime_guess::from_path;
use serde_json::json;
use tokio::fs::File;
use tokio_util::io::ReaderStream;
use uuid::Uuid;

use crate::core::app::AppState;
use crate::core::auth::{authenticate_with_query_token, check_permission, RecordAction};
use crate::core::db::{get_collection_by_name_or_id, list_collections};
use crate::core::env::Env;
use crate::core::realtime::RealtimeMessage;
use crate::core::record::{get_record, update_record};
use crate::core::schema::{Collection, FieldType};

type ApiError = (StatusCode, Json<serde_json::Value>);

/// Maximum size of a single uploaded file (`MAX_UPLOAD_MB`, default 50 MiB).
pub fn max_file_bytes() -> usize {
    Env::get_int("MAX_UPLOAD_MB", 50).max(1) as usize * 1024 * 1024
}

fn err(status: StatusCode, msg: impl Into<String>) -> ApiError {
    (
        status,
        Json(json!({ "code": status.as_u16(), "message": msg.into() })),
    )
}

fn internal(context: &str, e: impl std::fmt::Display) -> ApiError {
    tracing::error!("{}: {}", context, e);
    err(StatusCode::INTERNAL_SERVER_ERROR, format!("{}.", context))
}

/// A path segment may only contain `[A-Za-z0-9._-]`, never starts with `.`, and has no `..`.
/// This rejects separators, drive letters (`C:`), NUL bytes and every traversal trick.
fn is_safe_segment(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 255
        && !s.starts_with('.')
        && !s.contains("..")
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-' || c == '_')
}

fn invalid_path() -> ApiError {
    err(StatusCode::BAD_REQUEST, "Invalid file path.")
}

/// Generate a safe, unique filename preventing directory traversal
pub fn sanitize_filename(original: &str) -> String {
    let clean_name = std::path::Path::new(original)
        .file_name()
        .and_then(|f| f.to_str())
        .unwrap_or("file");

    let mut filtered: String = clean_name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '.' || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();

    // Keep names short (the tail keeps the extension) and never hidden / dot-only
    if filtered.len() > 100 {
        filtered = filtered[filtered.len() - 100..].to_string();
    }
    let filtered = filtered.trim_start_matches('.').to_string();

    let prefix = &Uuid::new_v4().simple().to_string()[..8];
    if filtered.is_empty() {
        format!("{}_attachment", prefix)
    } else {
        format!("{}_{}", prefix, filtered)
    }
}

/// Content types a browser would execute / render with scripting enabled.
fn is_active_content(mime: &str) -> bool {
    let m = mime.to_ascii_lowercase();
    m.contains("html")
        || m.contains("svg")
        || m.contains("xml")
        || m.contains("javascript")
        || m.contains("ecmascript")
}

fn is_public_rule(rule: Option<&str>) -> bool {
    match rule.map(str::trim) {
        None | Some("") => true,
        Some(r) => r.eq_ignore_ascii_case("public") || r == "*" || r.eq_ignore_ascii_case("anyone"),
    }
}

/// Only fields declared as `file` in the collection schema may receive uploaded file names.
/// Prevents overwriting arbitrary columns (`role`, `status`, ...) through a crafted field name.
fn is_file_field(col: &Collection, name: &str) -> bool {
    col.schema
        .iter()
        .any(|f| f.name == name && f.field_type == FieldType::File)
}

/// Download / serve static attachment file (with inline preview, download flag & browser caching)
pub async fn get_file(
    State(state): State<AppState>,
    Path((collection, record_id, filename)): Path<(String, String, String)>,
    axum::extract::Query(query): axum::extract::Query<std::collections::HashMap<String, String>>,
    headers: axum::http::HeaderMap,
) -> Result<Response, ApiError> {
    if !is_safe_segment(&collection) || !is_safe_segment(&record_id) || !is_safe_segment(&filename)
    {
        return Err(invalid_path());
    }

    let col = match get_collection_by_name_or_id(&state.db, &collection).await {
        Ok(Some(c)) => c,
        _ => {
            // Try plural or singular fallback
            let alt = if collection.ends_with('s') {
                collection[..collection.len() - 1].to_string()
            } else {
                format!("{}s", collection)
            };
            get_collection_by_name_or_id(&state.db, &alt)
                .await
                .map_err(|e| internal("Failed to load collection", e))?
                .ok_or_else(|| err(StatusCode::NOT_FOUND, "File not found."))?
        }
    };

    let auth = authenticate_with_query_token(&state, &headers, query.get("token").map(String::as_str)).await?;
    check_permission(RecordAction::View, &col, auth.as_ref(), None)?;

    let mut file_path = state
        .storage_dir
        .join(&col.name)
        .join(&record_id)
        .join(&filename);

    if !file_path.exists() {
        let alt_path = state
            .storage_dir
            .join(&collection)
            .join(&record_id)
            .join(&filename);
        if alt_path.exists() {
            file_path = alt_path;
        }
    }

    // Resolve symlinks and make sure we never leave the storage directory.
    let canonical = tokio::fs::canonicalize(&file_path)
        .await
        .map_err(|_| err(StatusCode::NOT_FOUND, "File not found."))?;
    let storage_root = tokio::fs::canonicalize(&state.storage_dir)
        .await
        .map_err(|e| internal("Storage directory unavailable", e))?;
    if !canonical.starts_with(&storage_root) || !canonical.is_file() {
        return Err(err(StatusCode::NOT_FOUND, "File not found."));
    }

    let file = File::open(&canonical)
        .await
        .map_err(|e| internal("Failed to open file", e))?;

    let guessed = from_path(&file_path).first_or_octet_stream();
    let active = is_active_content(guessed.as_ref());
    let content_type = if active {
        "application/octet-stream".to_string()
    } else {
        guessed.to_string()
    };

    let stream = ReaderStream::new(file);
    let body = Body::from_stream(stream);

    let is_download = query
        .get("download")
        .map(|v| v == "true" || v == "1")
        .unwrap_or(false);
    // Active content is always forced to download so it can never run in our origin.
    let disposition = if is_download || active {
        format!("attachment; filename=\"{}\"", filename)
    } else {
        format!("inline; filename=\"{}\"", filename)
    };

    let cache_control = if is_public_rule(col.view_rule.as_deref()) {
        "public, max-age=3600, stale-while-revalidate=86400"
    } else {
        "private, no-store"
    };

    let res = Response::builder()
        .header(
            header::CONTENT_TYPE,
            HeaderValue::from_str(&content_type)
                .unwrap_or(HeaderValue::from_static("application/octet-stream")),
        )
        .header(
            header::CONTENT_DISPOSITION,
            HeaderValue::from_str(&disposition).unwrap_or(HeaderValue::from_static("attachment")),
        )
        .header(header::CACHE_CONTROL, HeaderValue::from_static(cache_control))
        .header(
            header::X_CONTENT_TYPE_OPTIONS,
            HeaderValue::from_static("nosniff"),
        )
        .header(
            HeaderName::from_static("content-security-policy"),
            HeaderValue::from_static("default-src 'none'; sandbox"),
        )
        .body(body)
        .map_err(|e| internal("Failed to build response", e))?;

    Ok(res)
}

/// Upload one or more files for an existing record
pub async fn upload_file(
    State(state): State<AppState>,
    Path((collection_name, record_id)): Path<(String, String)>,
    headers: axum::http::HeaderMap,
    mut multipart: Multipart,
) -> Result<impl IntoResponse, ApiError> {
    if !is_safe_segment(&collection_name) || !is_safe_segment(&record_id) {
        return Err(invalid_path());
    }

    // 1. Validate collection
    let col = get_collection_by_name_or_id(&state.db, &collection_name)
        .await
        .map_err(|e| internal("Failed to load collection", e))?
        .ok_or_else(|| {
            err(
                StatusCode::NOT_FOUND,
                format!("Collection '{}' not found.", collection_name),
            )
        })?;

    // 2. Authenticate + coarse permission check BEFORE touching the record (no existence oracle)
    let auth = authenticate_with_query_token(&state, &headers, None).await?;
    check_permission(RecordAction::Update, &col, auth.as_ref(), None)?;

    let record = get_record(&state.db, &col, &record_id)
        .await
        .map_err(|e| internal("Failed to load record", e))?
        .ok_or_else(|| {
            err(
                StatusCode::NOT_FOUND,
                format!("Record '{}' not found in '{}'.", record_id, col.name),
            )
        })?;

    let existing_map = serde_json::to_value(&record)
        .ok()
        .and_then(|v| v.as_object().cloned());
    check_permission(RecordAction::Update, &col, auth.as_ref(), existing_map.as_ref())?;

    // 3. Ensure storage directory exists
    let rec_dir = state.storage_dir.join(&col.name).join(&record_id);
    tokio::fs::create_dir_all(&rec_dir)
        .await
        .map_err(|e| internal("Failed to create storage directory", e))?;

    let max_bytes = max_file_bytes();
    let mut uploaded_files = Vec::new();
    let mut update_payload = serde_json::Map::new();

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| err(StatusCode::BAD_REQUEST, format!("Invalid multipart body: {}", e)))?
    {
        let field_name = field.name().unwrap_or("file").to_string();
        let file_name = field.file_name().map(String::from);

        if let Some(orig_name) = file_name {
            if orig_name.trim().is_empty() {
                continue;
            }
            if !is_file_field(&col, &field_name) {
                return Err(err(
                    StatusCode::BAD_REQUEST,
                    format!("Field '{}' is not a file field of '{}'.", field_name, col.name),
                ));
            }
            let safe_name = sanitize_filename(&orig_name);
            let dest = rec_dir.join(&safe_name);

            let data = field
                .bytes()
                .await
                .map_err(|e| err(StatusCode::BAD_REQUEST, format!("Failed to read file bytes: {}", e)))?;

            let size = data.len();
            if size > max_bytes {
                return Err(err(
                    StatusCode::PAYLOAD_TOO_LARGE,
                    format!("File exceeds the maximum size of {} MB.", max_bytes / 1024 / 1024),
                ));
            }
            tokio::fs::write(&dest, &data)
                .await
                .map_err(|e| internal("Failed to save file", e))?;

            // If updating, delete the previous file from storage to avoid orphaned files
            if let Some(old_val) = record.data.get(&field_name).and_then(|v| v.as_str()) {
                if is_safe_segment(old_val) && old_val != safe_name {
                    let old_dest = rec_dir.join(old_val);
                    if old_dest.exists() {
                        let _ = tokio::fs::remove_file(&old_dest).await;
                    }
                }
            }

            let file_url = format!("/api/files/{}/{}/{}", col.name, record_id, safe_name);
            let mime = from_path(&safe_name).first_or_octet_stream().to_string();

            uploaded_files.push(json!({
                "field": field_name,
                "filename": safe_name,
                "original_name": orig_name,
                "size": size,
                "mime": mime,
                "url": file_url,
            }));

            update_payload.insert(field_name, serde_json::Value::String(safe_name));
        }
    }

    if uploaded_files.is_empty() {
        return Err(err(
            StatusCode::BAD_REQUEST,
            "No files provided in multipart request.",
        ));
    }

    // 4. Update the record column with the uploaded filename(s)
    let updated_record = if !update_payload.is_empty() {
        let rec = update_record(&state.db, &col, &record_id, update_payload)
            .await
            .map_err(|e| {
                err(
                    StatusCode::BAD_REQUEST,
                    format!("Failed to link file to record: {}", e),
                )
            })?;

        if col.realtime {
            state.realtime.broadcast(RealtimeMessage::new(
                "update",
                &col.name,
                serde_json::to_value(&rec).unwrap_or_default(),
            ));
        }

        rec
    } else {
        record
    };

    Ok((
        StatusCode::OK,
        Json(json!({
            "message": "File(s) uploaded successfully.",
            "uploaded": uploaded_files,
            "record": updated_record,
        })),
    ))
}

/// Delete an attachment file from storage and clear matching record column
pub async fn delete_file(
    State(state): State<AppState>,
    Path((collection_name, record_id, filename)): Path<(String, String, String)>,
    headers: axum::http::HeaderMap,
) -> Result<impl IntoResponse, ApiError> {
    if !is_safe_segment(&filename)
        || !is_safe_segment(&collection_name)
        || !is_safe_segment(&record_id)
    {
        return Err(invalid_path());
    }

    let col = get_collection_by_name_or_id(&state.db, &collection_name)
        .await
        .map_err(|e| internal("Failed to load collection", e))?
        .ok_or_else(|| {
            err(
                StatusCode::NOT_FOUND,
                format!("Collection '{}' not found.", collection_name),
            )
        })?;

    let auth = authenticate_with_query_token(&state, &headers, None).await?;
    check_permission(RecordAction::Update, &col, auth.as_ref(), None)?;

    let record = get_record(&state.db, &col, &record_id)
        .await
        .map_err(|e| internal("Failed to load record", e))?
        .ok_or_else(|| {
            err(
                StatusCode::NOT_FOUND,
                format!("Record '{}' not found in '{}'.", record_id, col.name),
            )
        })?;

    let existing_map = serde_json::to_value(&record)
        .ok()
        .and_then(|v| v.as_object().cloned());
    check_permission(RecordAction::Update, &col, auth.as_ref(), existing_map.as_ref())?;

    let file_path = state
        .storage_dir
        .join(&col.name)
        .join(&record_id)
        .join(&filename);

    if file_path.exists() {
        let _ = tokio::fs::remove_file(&file_path).await;
    }

    // Only clear columns that are declared file fields.
    let mut clear_payload = serde_json::Map::new();
    for (k, v) in &record.data {
        if v.as_str() == Some(filename.as_str()) && is_file_field(&col, k) {
            clear_payload.insert(k.clone(), serde_json::Value::Null);
        }
    }
    if !clear_payload.is_empty() {
        if let Err(e) = update_record(&state.db, &col, &record_id, clear_payload).await {
            tracing::warn!("failed to clear file column for {}/{}: {}", col.name, record_id, e);
        }
    }

    Ok(Json(json!({
        "success": true,
        "message": format!("File '{}' deleted successfully.", filename)
    })))
}

#[derive(Debug, serde::Deserialize)]
pub struct StorageListQuery {
    pub collection: Option<String>,
    pub search: Option<String>,
}

/// List all storage buckets and files across collections with aggregates
pub async fn list_storage(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    axum::extract::Query(query): axum::extract::Query<StorageListQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let auth = authenticate_with_query_token(&state, &headers, None).await?;
    if auth.is_none() {
        return Err(err(StatusCode::UNAUTHORIZED, "Authentication required."));
    }

    let storage_dir = state.storage_dir.clone();

    let (mut buckets_map, all_files, total_files, total_bytes) = tokio::task::spawn_blocking(move || {
        let mut buckets: std::collections::HashMap<String, (usize, u64)> = std::collections::HashMap::new();
        let mut files = Vec::new();
        let mut tot_files: usize = 0;
        let mut tot_bytes: u64 = 0;

        let search_dirs = vec![
            storage_dir.clone(),
            std::path::PathBuf::from("./padi_data/storage"),
            std::path::PathBuf::from("padi_data/storage"),
        ];

        let actual_dir = search_dirs.into_iter().find(|d| d.exists()).unwrap_or(storage_dir);

        if let Ok(col_entries) = std::fs::read_dir(&actual_dir) {
            for col_entry in col_entries.flatten() {
                let col_name = col_entry.file_name().to_string_lossy().to_string();
                if !is_safe_segment(&col_name) {
                    continue;
                }
                let col_path = col_entry.path();
                if !col_path.is_dir() {
                    continue;
                }

                buckets.entry(col_name.clone()).or_insert((0, 0));

                if let Ok(rec_entries) = std::fs::read_dir(&col_path) {
                    for rec_entry in rec_entries.flatten() {
                        let rec_id = rec_entry.file_name().to_string_lossy().to_string();
                        if !is_safe_segment(&rec_id) {
                            continue;
                        }
                        let rec_path = rec_entry.path();
                        if !rec_path.is_dir() {
                            continue;
                        }

                        if let Ok(file_entries) = std::fs::read_dir(&rec_path) {
                            for file_entry in file_entries.flatten() {
                                let file_name = file_entry.file_name().to_string_lossy().to_string();
                                if !is_safe_segment(&file_name) {
                                    continue;
                                }
                                if let Ok(metadata) = file_entry.metadata() {
                                    if !metadata.is_file() {
                                        continue;
                                    }
                                    let size = metadata.len();
                                    let modified = metadata
                                        .modified()
                                        .ok()
                                        .and_then(|t| {
                                            let duration = t.duration_since(std::time::UNIX_EPOCH).ok()?;
                                            Some(duration.as_millis() as i64)
                                        })
                                        .unwrap_or(0);

                                    let stat = buckets.entry(col_name.clone()).or_insert((0, 0));
                                    stat.0 += 1;
                                    stat.1 += size;

                                    tot_files += 1;
                                    tot_bytes += size;

                                    let mime = from_path(&file_name).first_or_octet_stream().to_string();
                                    let url = format!("/api/files/{}/{}/{}", col_name, rec_id, file_name);

                                    files.push(json!({
                                        "name": file_name,
                                        "collection": col_name,
                                        "record_id": rec_id,
                                        "size": size,
                                        "mime": mime,
                                        "modified_at": modified,
                                        "url": url,
                                    }));
                                }
                            }
                        }
                    }
                }
            }
        }
        (buckets, files, tot_files, tot_bytes)
    })
    .await
    .map_err(|e| internal("Failed to inspect storage directory", e))?;

    // Also register any collections that declare a File field but have no files yet
    if let Ok(collections) = list_collections(&state.db).await {
        for col in collections {
            if col.schema.iter().any(|f| f.field_type == FieldType::File) {
                buckets_map.entry(col.name.clone()).or_insert((0, 0));
            }
        }
    }

    let target_col = query.collection.as_deref().unwrap_or("").trim().to_lowercase();
    let search_q = query.search.as_deref().unwrap_or("").trim().to_lowercase();

    let mut filtered_files: Vec<serde_json::Value> = all_files
        .into_iter()
        .filter(|f| {
            if !target_col.is_empty() {
                let col = f.get("collection").and_then(|v| v.as_str()).unwrap_or("").to_lowercase();
                let match_exact = col == target_col;
                let match_p1 = col == format!("{}s", target_col);
                let match_p2 = target_col == format!("{}s", col);
                let match_p3 = col == format!("{}es", target_col);
                let match_p4 = target_col == format!("{}es", col);
                if !match_exact && !match_p1 && !match_p2 && !match_p3 && !match_p4 {
                    return false;
                }
            }

            if !search_q.is_empty() {
                let name = f.get("name").and_then(|v| v.as_str()).unwrap_or("").to_lowercase();
                let col = f.get("collection").and_then(|v| v.as_str()).unwrap_or("").to_lowercase();
                let rec = f.get("record_id").and_then(|v| v.as_str()).unwrap_or("").to_lowercase();
                if !name.contains(&search_q) && !col.contains(&search_q) && !rec.contains(&search_q) {
                    return false;
                }
            }

            true
        })
        .collect();

    // Sort files by modified_at desc
    filtered_files.sort_by(|a, b| {
        let ma = a.get("modified_at").and_then(|v| v.as_i64()).unwrap_or(0);
        let mb = b.get("modified_at").and_then(|v| v.as_i64()).unwrap_or(0);
        mb.cmp(&ma)
    });

    let mut buckets: Vec<serde_json::Value> = buckets_map
        .into_iter()
        .map(|(name, (count, size))| {
            json!({
                "name": name,
                "file_count": count,
                "total_bytes": size,
            })
        })
        .collect();
    buckets.sort_by(|a, b| {
        let na = a.get("name").and_then(|v| v.as_str()).unwrap_or("");
        let nb = b.get("name").and_then(|v| v.as_str()).unwrap_or("");
        na.cmp(nb)
    });

    Ok(Json(json!({
        "total_files": total_files,
        "total_bytes": total_bytes,
        "buckets": buckets,
        "files": filtered_files,
    })))
}
