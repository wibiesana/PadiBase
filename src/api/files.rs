use axum::{
    body::Body,
    extract::{Multipart, Path, State},
    http::{header, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use mime_guess::from_path;
use serde_json::json;
use tokio::fs::File;
use tokio_util::io::ReaderStream;
use uuid::Uuid;

use crate::core::app::AppState;
use crate::core::auth::{authenticate_headers, check_permission, RecordAction};
use crate::core::db::get_collection_by_name_or_id;
use crate::core::record::{get_record, update_record};
use crate::core::realtime::RealtimeMessage;

/// Generate a safe, unique filename preventing directory traversal
pub fn sanitize_filename(original: &str) -> String {
    let clean_name = std::path::Path::new(original)
        .file_name()
        .and_then(|f| f.to_str())
        .unwrap_or("file");

    let filtered: String = clean_name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '.' || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();

    let prefix = &Uuid::new_v4().simple().to_string()[..8];
    if filtered.is_empty() || filtered == "." || filtered == ".." {
        format!("{}_attachment", prefix)
    } else {
        format!("{}_{}", prefix, filtered)
    }
}

/// Download / serve static attachment file (with inline preview, download flag & browser caching)
pub async fn get_file(
    State(state): State<AppState>,
    Path((collection, record_id, filename)): Path<(String, String, String)>,
    axum::extract::Query(query): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Result<Response, (StatusCode, Json<serde_json::Value>)> {
    // Sanitize parameters to avoid path traversal
    if filename.contains("..") || collection.contains("..") || record_id.contains("..") {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({ "code": 400, "message": "Invalid file path." })),
        ));
    }

    let file_path = state
        .storage_dir
        .join(&collection)
        .join(&record_id)
        .join(&filename);

    if !file_path.exists() {
        return Err((
            StatusCode::NOT_FOUND,
            Json(json!({ "code": 404, "message": "File not found." })),
        ));
    }

    let file = File::open(&file_path).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "code": 500, "message": e.to_string() })),
        )
    })?;

    let mime = from_path(&file_path).first_or_octet_stream();
    let stream = ReaderStream::new(file);
    let body = Body::from_stream(stream);

    let is_download = query.get("download").map(|v| v == "true" || v == "1").unwrap_or(false);
    let disposition = if is_download {
        format!("attachment; filename=\"{}\"", filename)
    } else {
        format!("inline; filename=\"{}\"", filename)
    };

    let res = Response::builder()
        .header(
            header::CONTENT_TYPE,
            HeaderValue::from_str(mime.as_ref())
                .unwrap_or(HeaderValue::from_static("application/octet-stream")),
        )
        .header(
            header::CONTENT_DISPOSITION,
            HeaderValue::from_str(&disposition)
                .unwrap_or(HeaderValue::from_static("inline")),
        )
        .header(
            header::CACHE_CONTROL,
            HeaderValue::from_static("public, max-age=604800, stale-while-revalidate=86400"),
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

/// Upload one or more files for an existing record
pub async fn upload_file(
    State(state): State<AppState>,
    Path((collection_name, record_id)): Path<(String, String)>,
    headers: axum::http::HeaderMap,
    mut multipart: Multipart,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    // 1. Validate collection
    let col = get_collection_by_name_or_id(&state.db, &collection_name)
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
                Json(json!({ "code": 404, "message": format!("Collection '{}' not found.", collection_name) })),
            )
        })?;

    // 2. Validate record exists
    let record = get_record(&state.db, &col, &record_id)
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
                Json(json!({ "code": 404, "message": format!("Record '{}' not found in '{}'.", record_id, col.name) })),
            )
        })?;

    let auth = authenticate_headers(&headers)?;
    let existing_map = serde_json::to_value(&record)
        .ok()
        .and_then(|v| v.as_object().cloned());
    check_permission(RecordAction::Update, &col, auth.as_ref(), existing_map.as_ref())?;

    // 3. Ensure storage directory exists
    let rec_dir = state.storage_dir.join(&col.name).join(&record_id);
    tokio::fs::create_dir_all(&rec_dir).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "code": 500, "message": format!("Failed to create storage directory: {}", e) })),
        )
    })?;

    let mut uploaded_files = Vec::new();
    let mut update_payload = serde_json::Map::new();

    while let Ok(Some(field)) = multipart.next_field().await {
        let field_name = field.name().unwrap_or("file").to_string();
        let file_name = field.file_name().map(String::from);

        if let Some(orig_name) = file_name {
            if orig_name.trim().is_empty() {
                continue;
            }
            let safe_name = sanitize_filename(&orig_name);
            let dest = rec_dir.join(&safe_name);

            let data = field.bytes().await.map_err(|e| {
                (
                    StatusCode::BAD_REQUEST,
                    Json(json!({ "code": 400, "message": format!("Failed to read file bytes: {}", e) })),
                )
            })?;

            let size = data.len();
            tokio::fs::write(&dest, &data).await.map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({ "code": 500, "message": format!("Failed to save file: {}", e) })),
                )
            })?;

            // If updating, delete the previous file from storage to avoid orphaned files
            if let Some(old_val) = record.data.get(&field_name).and_then(|v| v.as_str()) {
                if !old_val.is_empty() && old_val != safe_name {
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
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({ "code": 400, "message": "No files provided in multipart request." })),
        ));
    }

    // 4. Update the record column with the uploaded filename(s)
    let updated_record = if !update_payload.is_empty() {
        let rec = update_record(&state.db, &col, &record_id, update_payload)
            .await
            .map_err(|e| {
                (
                    StatusCode::BAD_REQUEST,
                    Json(json!({ "code": 400, "message": format!("Failed to link file to record: {}", e) })),
                )
            })?;

        if col.realtime {
            state.realtime.broadcast(RealtimeMessage {
                action: "update".to_string(),
                collection: col.name.clone(),
                record: serde_json::to_value(&rec).unwrap_or_default(),
            });
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
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    if filename.contains("..") || collection_name.contains("..") || record_id.contains("..") {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({ "code": 400, "message": "Invalid file path." })),
        ));
    }

    let col = get_collection_by_name_or_id(&state.db, &collection_name)
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
                Json(json!({ "code": 404, "message": format!("Collection '{}' not found.", collection_name) })),
            )
        })?;

    let record = get_record(&state.db, &col, &record_id)
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
                Json(json!({ "code": 404, "message": format!("Record '{}' not found in '{}'.", record_id, col.name) })),
            )
        })?;

    let auth = authenticate_headers(&headers)?;
    let existing_map = serde_json::to_value(&record)
        .ok()
        .and_then(|v| v.as_object().cloned());
    check_permission(RecordAction::Update, &col, auth.as_ref(), existing_map.as_ref())?;

    let file_path = state
        .storage_dir
        .join(&collection_name)
        .join(&record_id)
        .join(&filename);

    if file_path.exists() {
        let _ = tokio::fs::remove_file(&file_path).await;
    }

    let mut clear_payload = serde_json::Map::new();
    for (k, v) in &record.data {
        if v.as_str() == Some(&filename) {
            clear_payload.insert(k.clone(), serde_json::Value::Null);
        }
    }
    if !clear_payload.is_empty() {
        let _ = update_record(&state.db, &col, &record_id, clear_payload).await;
    }

    Ok(Json(json!({
        "success": true,
        "message": format!("File '{}' deleted successfully.", filename)
    })))
}
