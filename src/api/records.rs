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
use crate::core::auth::{authenticate_headers, check_permission, RecordAction};
use crate::core::db::get_collection_by_name_or_id;
use crate::core::realtime::RealtimeMessage;
use crate::core::record::{
    create_record, delete_record, get_all_records, get_all_records_with, get_record,
    get_record_with, list_records_with, update_record,
};

#[derive(Deserialize, Default)]
pub struct SingleParams {
    pub with: Option<String>,
    pub expand: Option<String>,
    pub auto_load: Option<bool>,
}

#[derive(Deserialize, Default)]
pub struct ListParams {
    pub page: Option<i64>,
    #[serde(rename = "perPage")]
    pub per_page: Option<i64>,
    pub sort: Option<String>,
    pub filter: Option<String>,
    pub all: Option<bool>,
    pub with: Option<String>,
    pub expand: Option<String>,
    pub auto_load: Option<bool>,
}

#[derive(Deserialize, Default)]
pub struct AllParams {
    pub sort: Option<String>,
    #[serde(rename = "sortBy")]
    pub sort_by: Option<String>,
    #[serde(rename = "sort_by")]
    pub sort_by_snake: Option<String>,
    #[serde(rename = "orderBy")]
    pub order_by: Option<String>,
    #[serde(rename = "order_by")]
    pub order_by_snake: Option<String>,
    pub order: Option<String>,
    pub filter: Option<String>,
    pub search: Option<String>,
    pub q: Option<String>,
    pub with: Option<String>,
    pub expand: Option<String>,
    pub auto_load: Option<bool>,
}

pub fn parse_with_param(
    with: Option<&str>,
    expand: Option<&str>,
    auto_load: Option<bool>,
) -> Option<Vec<String>> {
    if let Some(false) = auto_load {
        return Some(vec!["none".to_string()]);
    }
    if let Some(raw) = with.or(expand) {
        if !raw.trim().is_empty() {
            let list: Vec<String> = raw
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
            if !list.is_empty() {
                return Some(list);
            }
        }
    }
    if let Some(true) = auto_load {
        return Some(vec!["*".to_string()]);
    }
    None
}

pub fn resolve_all_sort_and_filter(params: &AllParams) -> (Option<String>, Option<String>) {
    let sort_clause = if let Some(ref s) = params.sort {
        Some(s.clone())
    } else {
        let sort_col = params
            .sort_by
            .as_deref()
            .or(params.sort_by_snake.as_deref())
            .or(params.order_by.as_deref())
            .or(params.order_by_snake.as_deref());
        if let Some(col_name) = sort_col {
            let is_desc = params
                .order
                .as_deref()
                .map(|o| o.eq_ignore_ascii_case("desc"))
                .unwrap_or(false);
            if is_desc {
                Some(format!("-{}", col_name))
            } else {
                Some(format!("+{}", col_name))
            }
        } else {
            None
        }
    };

    let filter_clause = if let Some(ref f) = params.filter {
        Some(f.clone())
    } else if let Some(ref s) = params.search {
        if !s.trim().is_empty() {
            Some(s.clone())
        } else {
            None
        }
    } else if let Some(ref q) = params.q {
        if !q.trim().is_empty() {
            Some(q.clone())
        } else {
            None
        }
    } else {
        None
    };

    (sort_clause, filter_clause)
}

pub async fn list(
    State(state): State<AppState>,
    Path(collection_name): Path<String>,
    Query(params): Query<ListParams>,
    headers: axum::http::HeaderMap,
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

    let auth = authenticate_headers(&headers)?;
    check_permission(RecordAction::List, &col, auth.as_ref(), None)?;

    let with_vec = parse_with_param(params.with.as_deref(), params.expand.as_deref(), params.auto_load);
    let with_refs: Option<Vec<&str>> = with_vec.as_ref().map(|v| v.iter().map(String::as_str).collect());

    if params.all.unwrap_or(false) || params.per_page == Some(-1) || params.per_page == Some(0) {
        let records = get_all_records_with(
            &state.db,
            &col,
            params.sort.as_deref(),
            params.filter.as_deref(),
            with_refs.as_deref(),
        )
        .await
        .map_err(|e| {
            (
                StatusCode::BAD_REQUEST,
                Json(json!({ "code": 400, "message": format!("Query error: {}", e) })),
            )
        })?;

        let total = records.len() as i64;
        return Ok(Json(json!({
            "page": 1,
            "perPage": total,
            "totalItems": total,
            "totalPages": 1,
            "items": records,
        })));
    }

    let res = list_records_with(
        &state.db,
        &col,
        params.page.unwrap_or(1),
        params.per_page.unwrap_or(30),
        params.sort.as_deref(),
        params.filter.as_deref(),
        with_refs.as_deref(),
    )
    .await
    .map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            Json(json!({ "code": 400, "message": format!("Query error: {}", e) })),
        )
    })?;

    Ok(Json(json!(res)))
}

pub async fn all(
    State(state): State<AppState>,
    Path(collection_name): Path<String>,
    Query(params): Query<AllParams>,
    headers: axum::http::HeaderMap,
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

    let auth = authenticate_headers(&headers)?;
    check_permission(RecordAction::List, &col, auth.as_ref(), None)?;

    let (sort_clause, filter_clause) = resolve_all_sort_and_filter(&params);
    let with_vec = parse_with_param(params.with.as_deref(), params.expand.as_deref(), params.auto_load);
    let with_refs: Option<Vec<&str>> = with_vec.as_ref().map(|v| v.iter().map(String::as_str).collect());

    let records = get_all_records_with(
        &state.db,
        &col,
        sort_clause.as_deref(),
        filter_clause.as_deref(),
        with_refs.as_deref(),
    )
    .await
    .map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            Json(json!({ "code": 400, "message": format!("Query error: {}", e) })),
        )
    })?;

    Ok(Json(json!(records)))
}

pub async fn get_one(
    State(state): State<AppState>,
    Path((collection_name, id)): Path<(String, String)>,
    Query(params): Query<SingleParams>,
    headers: axum::http::HeaderMap,
) -> Result<Response, (StatusCode, Json<Value>)> {
    if id.eq_ignore_ascii_case("all") {
        return all(State(state), Path(collection_name), Query(AllParams::default()), headers)
            .await
            .map(|r| r.into_response());
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
                Json(json!({ "code": 404, "message": "Collection not found." })),
            )
        })?;

    let auth = authenticate_headers(&headers)?;
    check_permission(RecordAction::View, &col, auth.as_ref(), None)?;

    let with_vec = parse_with_param(params.with.as_deref(), params.expand.as_deref(), params.auto_load);
    let with_refs: Option<Vec<&str>> = with_vec.as_ref().map(|v| v.iter().map(String::as_str).collect());

    let record = get_record_with(&state.db, &col, &id, with_refs.as_deref())
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

    Ok(Json(record).into_response())
}

async fn parse_request_payload(
    req: axum::extract::Request,
    state: &AppState,
) -> Result<(Map<String, Value>, Vec<(String, String, Vec<u8>)>), (StatusCode, Json<Value>)> {
    let is_multipart = req
        .headers()
        .get(axum::http::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.starts_with("multipart/form-data"))
        .unwrap_or(false);

    if is_multipart {
        use axum::extract::FromRequest;
        let mut multipart = axum::extract::Multipart::from_request(req, state)
            .await
            .map_err(|e| {
                (
                    StatusCode::BAD_REQUEST,
                    Json(json!({ "code": 400, "message": format!("Invalid multipart form: {}", e) })),
                )
            })?;

        let mut payload = Map::new();
        let mut files = Vec::new();

        while let Ok(Some(field)) = multipart.next_field().await {
            let field_name = field.name().unwrap_or("").to_string();
            if field_name.is_empty() {
                continue;
            }

            let file_name = field.file_name().map(String::from);
            if let Some(orig_name) = file_name {
                if !orig_name.trim().is_empty() {
                    let safe_name = crate::api::files::sanitize_filename(&orig_name);
                    if let Ok(bytes) = field.bytes().await {
                        files.push((field_name.clone(), safe_name.clone(), bytes.to_vec()));
                        payload.insert(field_name, Value::String(safe_name));
                    }
                }
            } else if let Ok(text) = field.text().await {
                let val = if text.eq_ignore_ascii_case("true") {
                    Value::Bool(true)
                } else if text.eq_ignore_ascii_case("false") {
                    Value::Bool(false)
                } else if text.eq_ignore_ascii_case("null") {
                    Value::Null
                } else if let Ok(n) = text.parse::<i64>() {
                    Value::from(n)
                } else if let Ok(f) = text.parse::<f64>() {
                    Value::from(f)
                } else if text.starts_with('{') || text.starts_with('[') {
                    serde_json::from_str(&text).unwrap_or(Value::String(text))
                } else {
                    Value::String(text)
                };
                payload.insert(field_name, val);
            }
        }

        Ok((payload, files))
    } else {
        let bytes = axum::body::to_bytes(req.into_body(), 64 * 1024 * 1024)
            .await
            .map_err(|e| {
                (
                    StatusCode::BAD_REQUEST,
                    Json(json!({ "code": 400, "message": format!("Failed to read request body: {}", e) })),
                )
            })?;

        let payload: Map<String, Value> = if bytes.is_empty() {
            Map::new()
        } else {
            serde_json::from_slice(&bytes).map_err(|e| {
                (
                    StatusCode::BAD_REQUEST,
                    Json(json!({ "code": 400, "message": format!("Invalid JSON payload: {}", e) })),
                )
            })?
        };

        Ok((payload, Vec::new()))
    }
}

pub async fn create(
    State(state): State<AppState>,
    Path(collection_name): Path<String>,
    req: axum::extract::Request,
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

    let auth = authenticate_headers(req.headers())?;
    check_permission(RecordAction::Create, &col, auth.as_ref(), None)?;

    let (payload, pending_files) = parse_request_payload(req, &state).await?;

    let mut record = create_record(&state.db, &col, payload)
        .await
        .map_err(|e| {
            (
                StatusCode::BAD_REQUEST,
                Json(json!({ "code": 400, "message": format!("Failed to create record: {}", e) })),
            )
        })?;

    // Save pending uploaded files to storage
    if !pending_files.is_empty() {
        let rec_id = record.id.clone();
        let rec_dir = state.storage_dir.join(&col.name).join(&rec_id);
        let _ = tokio::fs::create_dir_all(&rec_dir).await;
        let mut file_updates = Map::new();

        for (field_name, safe_name, data) in pending_files {
            let dest = rec_dir.join(&safe_name);
            let _ = tokio::fs::write(&dest, &data).await;
            file_updates.insert(field_name, Value::String(safe_name));
        }

        if !file_updates.is_empty() {
            if let Ok(updated) = update_record(&state.db, &col, &rec_id, file_updates).await {
                record = updated;
            }
        }
    }

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
    req: axum::extract::Request,
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

    let auth = authenticate_headers(req.headers())?;

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

    let existing_map = serde_json::to_value(&existing)
        .ok()
        .and_then(|v| v.as_object().cloned());

    check_permission(
        RecordAction::Update,
        &col,
        auth.as_ref(),
        existing_map.as_ref(),
    )?;

    let (mut payload, pending_files) = parse_request_payload(req, &state).await?;

    // Save any uploaded files directly to storage for this record
    if !pending_files.is_empty() {
        let rec_dir = state.storage_dir.join(&col.name).join(&id);
        let _ = tokio::fs::create_dir_all(&rec_dir).await;

        for (field_name, safe_name, data) in pending_files {
            let dest = rec_dir.join(&safe_name);
            let _ = tokio::fs::write(&dest, &data).await;
            payload.insert(field_name, Value::String(safe_name));
        }
    }

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
    headers: axum::http::HeaderMap,
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

    let auth = authenticate_headers(&headers)?;

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

    let existing_map = serde_json::to_value(&existing)
        .ok()
        .and_then(|v| v.as_object().cloned());

    check_permission(
        RecordAction::Delete,
        &col,
        auth.as_ref(),
        existing_map.as_ref(),
    )?;

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

    let items = get_all_records(&state.db, &col, Some("+created"), None)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "code": 500, "message": e.to_string() })),
            )
        })?;

    let json_bytes = serde_json::to_vec_pretty(&items).map_err(|e| {
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
    headers: axum::http::HeaderMap,
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

    let auth = authenticate_headers(&headers)?;
    check_permission(RecordAction::Create, &col, auth.as_ref(), None)?;

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
