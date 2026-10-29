use axum::{
    body::Body,
    extract::{Path, State},
    http::{header, HeaderValue, StatusCode},
    response::Response,
    Json,
};
use mime_guess::from_path;
use serde_json::json;
use tokio::fs::File;
use tokio_util::io::ReaderStream;

use crate::core::app::AppState;

pub async fn get_file(
    State(state): State<AppState>,
    Path((collection, record_id, filename)): Path<(String, String, String)>,
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

    let res = Response::builder()
        .header(
            header::CONTENT_TYPE,
            HeaderValue::from_str(mime.as_ref()).unwrap_or(HeaderValue::from_static("application/octet-stream")),
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
