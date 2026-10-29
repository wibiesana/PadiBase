use axum::{
    body::Body,
    extract::{Path, Query, State},
    http::{header, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::Deserialize;
use serde_json::{json, Map, Value};

use crate::core::app::AppState;
use crate::core::db::get_collection_by_name_or_id;
use crate::core::realtime::RealtimeMessage;
use crate::core::record::{create_record, delete_record, get_record, list_records, update_record};

#[derive(Deserialize)]
pub struct ListParams {
    pub page: Option<i64>,
    #[serde(rename = "perPage")]
    pub per_page: Option<i64>,
    pub sort: Option<String>,
    pub filter: Option<String>,
}

pub async fn list(
    State(state): State<AppState>,
    Path(collection_name): Path<String>,
    Query(params): Query<ListParams>,
) -> Result<impl IntoResponse, (StatusCode, Json<Value>)> {
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
                Json(json!({ "code": 404, "message": "Collection not found." })),
            )
        })?;

    let res = list_records(
        &state.db,
        &col,
        params.page.unwrap_or(1),
        params.per_page.unwrap_or(30),
        params.sort.as_deref(),
        params.filter.as_deref(),
    )
    .await
    .map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            Json(json!({ "code": 400, "message": format!("Query error: {}", e) })),
        )
    })?;

    Ok(Json(res))
}

pub async fn get_one(
    State(state): State<AppState>,
    Path((collection_name, id)): Path<(String, String)>,
) -> Result<impl IntoResponse, (StatusCode, Json<Value>)> {
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
                Json(json!({ "code": 404, "message": "Collection not found." })),
            )
        })?;

    let record = get_record(&state.db, &col, &id)
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
                Json(json!({ "code": 404, "message": "Record not found." })),
            )
        })?;

    Ok(Json(record))
}

pub async fn create(
    State(state): State<AppState>,
    Path(collection_name): Path<String>,
    Json(payload): Json<Map<String, Value>>,
) -> Result<impl IntoResponse, (StatusCode, Json<Value>)> {
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
                Json(json!({ "code": 404, "message": "Collection not found." })),
            )
        })?;

    let record = create_record(&state.db, &col, payload)
        .await
        .map_err(|e| {
            (
                StatusCode::BAD_REQUEST,
                Json(json!({ "code": 400, "message": format!("Failed to create record: {}", e) })),
            )
        })?;

    // Realtime broadcast event (only if enabled on this collection)
    if col.realtime {
        state.realtime.broadcast(RealtimeMessage {
            action: "create".to_string(),
            collection: col.name.clone(),
            record: serde_json::to_value(&record).unwrap_or_default(),
        });
    }

    Ok((StatusCode::CREATED, Json(record)))
}

pub async fn update(
    State(state): State<AppState>,
    Path((collection_name, id)): Path<(String, String)>,
    Json(payload): Json<Map<String, Value>>,
) -> Result<impl IntoResponse, (StatusCode, Json<Value>)> {
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
                Json(json!({ "code": 404, "message": "Collection not found." })),
            )
        })?;

    let record = update_record(&state.db, &col, &id, payload)
        .await
        .map_err(|e| {
            (
                StatusCode::BAD_REQUEST,
                Json(json!({ "code": 400, "message": format!("Failed to update record: {}", e) })),
            )
        })?;

    // Realtime broadcast event (only if enabled on this collection)
    if col.realtime {
        state.realtime.broadcast(RealtimeMessage {
            action: "update".to_string(),
            collection: col.name.clone(),
            record: serde_json::to_value(&record).unwrap_or_default(),
        });
    }

    Ok(Json(record))
}

pub async fn delete(
    State(state): State<AppState>,
    Path((collection_name, id)): Path<(String, String)>,
) -> Result<impl IntoResponse, (StatusCode, Json<Value>)> {
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
                Json(json!({ "code": 404, "message": "Collection not found." })),
            )
        })?;

    let existing = get_record(&state.db, &col, &id)
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
                Json(json!({ "code": 404, "message": "Record not found." })),
            )
        })?;

    let deleted = delete_record(&state.db, &col, &id).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "code": 500, "message": e.to_string() })),
        )
    })?;

    if deleted {
        // Realtime broadcast event (only if enabled on this collection)
        if col.realtime {
            state.realtime.broadcast(RealtimeMessage {
                action: "delete".to_string(),
                collection: col.name.clone(),
                record: serde_json::to_value(&existing).unwrap_or_default(),
            });
        }

        Ok(StatusCode::NO_CONTENT)
    } else {
        Err((
            StatusCode::NOT_FOUND,
            Json(json!({ "code": 404, "message": "Record not found." })),
        ))
    }
}

pub async fn export_records(
    State(state): State<AppState>,
    Path(collection_name): Path<String>,
) -> Result<Response, (StatusCode, Json<Value>)> {
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
                Json(json!({ "code": 404, "message": "Collection not found." })),
            )
        })?;

    let res = list_records(&state.db, &col, 1, 100_000, Some("+created"), None)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "code": 500, "message": e.to_string() })),
            )
        })?;

    let json_bytes = serde_json::to_vec_pretty(&res.items).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "code": 500, "message": e.to_string() })),
        )
    })?;

    let filename = format!("{}_export.json", col.name);
    let disposition = format!("attachment; filename=\"{}\"", filename);

    let response = Response::builder()
        .header(header::CONTENT_TYPE, HeaderValue::from_static("application/json"))
        .header(
            header::CONTENT_DISPOSITION,
            HeaderValue::from_str(&disposition).unwrap_or(HeaderValue::from_static("attachment")),
        )
        .body(Body::from(json_bytes))
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "code": 500, "message": e.to_string() })),
            )
        })?;

    Ok(response)
}

pub async fn import_records(
    State(state): State<AppState>,
    Path(collection_name): Path<String>,
    Json(items): Json<Vec<Map<String, Value>>>,
) -> Result<impl IntoResponse, (StatusCode, Json<Value>)> {
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
                Json(json!({ "code": 404, "message": "Collection not found." })),
            )
        })?;

    let mut imported_count = 0;
    let mut errors = Vec::new();

    for (index, payload) in items.into_iter().enumerate() {
        match create_record(&state.db, &col, payload).await {
            Ok(rec) => {
                imported_count += 1;
                if col.realtime {
                    state.realtime.broadcast(RealtimeMessage {
                        action: "create".to_string(),
                        collection: col.name.clone(),
                        record: serde_json::to_value(&rec).unwrap_or_default(),
                    });
                }
            }
            Err(e) => {
                errors.push(format!("Row #{}: {}", index + 1, e));
            }
        }
    }

    Ok(Json(json!({
        "success": true,
        "imported": imported_count,
        "errors": errors,
        "message": format!("Successfully imported {} records", imported_count)
    })))
}
