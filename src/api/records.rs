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
    count_records, create_record, delete_record, delete_records_batch, get_all_records,
    get_all_records_with_limit, get_record, get_record_with, list_records_with,
    project_record_fields, update_record,
};

#[derive(Deserialize, Default)]
pub struct SingleParams {
    pub with: Option<String>,
    pub expand: Option<String>,
    pub auto_load: Option<bool>,
    pub limit: Option<usize>,
    pub fields: Option<String>,
    pub select: Option<String>,
    pub filter: Option<String>,
    pub search: Option<String>,
    pub q: Option<String>,
    pub sort: Option<String>,
}

#[derive(Deserialize, Default)]
pub struct ListParams {
    pub page: Option<i64>,
    #[serde(rename = "perPage")]
    pub per_page: Option<i64>,
    #[serde(rename = "per_page")]
    pub per_page_snake: Option<i64>,
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
    pub all: Option<bool>,
    pub with: Option<String>,
    pub expand: Option<String>,
    pub auto_load: Option<bool>,
    pub limit: Option<usize>,
    pub fields: Option<String>,
    pub select: Option<String>,
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
    pub limit: Option<usize>,
    #[serde(rename = "perPage")]
    pub per_page_camel: Option<usize>,
    #[serde(rename = "per_page")]
    pub per_page: Option<usize>,
    pub offset: Option<usize>,
    pub skip: Option<usize>,
    pub fields: Option<String>,
    pub select: Option<String>,
}

pub fn parse_fields_param(fields: Option<&str>, select: Option<&str>) -> Option<Vec<String>> {
    let raw = fields.or(select)?;
    if raw.trim().is_empty() {
        return None;
    }
    let list: Vec<String> = raw
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    if list.is_empty() {
        None
    } else {
        Some(list)
    }
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

pub async fn validate_role_if_present(
    db: &crate::core::db::DbPool,
    payload: &Map<String, Value>,
) -> Result<(), (StatusCode, Json<Value>)> {
    if let Some(role_val) = payload.get("role") {
        if let Some(role_str) = role_val.as_str() {
            let trimmed = role_str.trim();
            if !trimmed.is_empty() {
                let valid_roles = crate::core::db::get_available_roles(db).await;
                if !valid_roles.iter().any(|r| r.eq_ignore_ascii_case(trimmed)) {
                    return Err((
                        StatusCode::BAD_REQUEST,
                        Json(json!({
                            "code": 400,
                            "message": format!(
                                "Role '{}' is not available. Available roles are: {}",
                                trimmed,
                                valid_roles.join(", ")
                            )
                        })),
                    ));
                }
            }
        }
    }
    Ok(())
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

pub fn resolve_list_sort_and_filter(params: &ListParams) -> (Option<String>, Option<String>) {
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

    let (sort_clause, filter_clause) = resolve_list_sort_and_filter(&params);
    let with_vec = parse_with_param(params.with.as_deref(), params.expand.as_deref(), params.auto_load);
    let with_refs: Option<Vec<&str>> = with_vec.as_ref().map(|v| v.iter().map(String::as_str).collect());
    let fields_vec = parse_fields_param(params.fields.as_deref(), params.select.as_deref());
    let fields_refs: Option<Vec<&str>> = fields_vec.as_ref().map(|v| v.iter().map(String::as_str).collect());

    let effective_per_page = params
        .per_page
        .or(params.per_page_snake)
        .or(params.limit.map(|l| l as i64));

    if params.all.unwrap_or(false) || effective_per_page == Some(-1) || effective_per_page == Some(0) {
        let requested_limit = params.limit;
        let effective_limit = match requested_limit {
            Some(l) if l > 0 => std::cmp::min(l, 5000),
            _ => 5000,
        };

        let mut records = get_all_records_with_limit(
            &state.db,
            &col,
            sort_clause.as_deref(),
            filter_clause.as_deref(),
            with_refs.as_deref(),
            Some(effective_limit),
            None,
        )
        .await
        .map_err(|e| {
            (
                StatusCode::BAD_REQUEST,
                Json(json!({ "code": 400, "message": format!("Query error: {}", e) })),
            )
        })?;

        if let Some(ref f_refs) = fields_refs {
            for rec in &mut records {
                project_record_fields(rec, Some(f_refs));
            }
        }

        let total = records.len() as i64;
        return Ok(Json(json!({
            "page": 1,
            "perPage": total,
            "totalItems": total,
            "totalPages": 1,
            "items": records,
        })));
    }

    let mut res = list_records_with(
        &state.db,
        &col,
        params.page.unwrap_or(1),
        effective_per_page.unwrap_or(30),
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

    if let Some(ref f_refs) = fields_refs {
        for rec in &mut res.items {
            project_record_fields(rec, Some(f_refs));
        }
    }

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
    let fields_vec = parse_fields_param(params.fields.as_deref(), params.select.as_deref());
    let fields_refs: Option<Vec<&str>> = fields_vec.as_ref().map(|v| v.iter().map(String::as_str).collect());

    // Capped at maximum 5000 records to prevent excessive data fetching, while allowing user to specify a custom limit <= 5000
    let requested_limit = params.limit.or(params.per_page).or(params.per_page_camel);
    let effective_limit = match requested_limit {
        Some(l) if l > 0 => std::cmp::min(l, 5000),
        _ => 5000,
    };
    let offset = params.offset.or(params.skip);

    let mut records = get_all_records_with_limit(
        &state.db,
        &col,
        sort_clause.as_deref(),
        filter_clause.as_deref(),
        with_refs.as_deref(),
        Some(effective_limit),
        offset,
    )
    .await
    .map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            Json(json!({ "code": 400, "message": format!("Query error: {}", e) })),
        )
    })?;

    if let Some(ref f_refs) = fields_refs {
        for rec in &mut records {
            project_record_fields(rec, Some(f_refs));
        }
    }

    Ok(Json(json!(records)))
}

pub async fn count(
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

    let (_, filter_clause) = resolve_all_sort_and_filter(&params);
    let total = count_records(&state.db, &col, filter_clause.as_deref())
        .await
        .map_err(|e| {
            (
                StatusCode::BAD_REQUEST,
                Json(json!({ "code": 400, "message": format!("Query error: {}", e) })),
            )
        })?;

    Ok(Json(json!({
        "collection": col.name,
        "count": total,
        "total": total
    })))
}

pub async fn first(
    State(state): State<AppState>,
    Path(collection_name): Path<String>,
    Query(params): Query<AllParams>,
    headers: axum::http::HeaderMap,
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

    let auth = authenticate_headers(&headers)?;
    check_permission(RecordAction::View, &col, auth.as_ref(), None)?;

    let (sort_clause, filter_clause) = resolve_all_sort_and_filter(&params);
    let with_vec = parse_with_param(params.with.as_deref(), params.expand.as_deref(), params.auto_load);
    let with_refs: Option<Vec<&str>> = with_vec.as_ref().map(|v| v.iter().map(String::as_str).collect());
    let fields_vec = parse_fields_param(params.fields.as_deref(), params.select.as_deref());
    let fields_refs: Option<Vec<&str>> = fields_vec.as_ref().map(|v| v.iter().map(String::as_str).collect());

    let mut records = get_all_records_with_limit(
        &state.db,
        &col,
        sort_clause.as_deref(),
        filter_clause.as_deref(),
        with_refs.as_deref(),
        Some(1),
        None,
    )
    .await
    .map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            Json(json!({ "code": 400, "message": format!("Query error: {}", e) })),
        )
    })?;

    if let Some(mut record) = records.pop() {
        if let Some(ref f_refs) = fields_refs {
            project_record_fields(&mut record, Some(f_refs));
        }
        Ok(Json(record).into_response())
    } else {
        Err((
            StatusCode::NOT_FOUND,
            Json(json!({ "code": 404, "message": "Record not found." })),
        ))
    }
}

pub async fn get_one(
    State(state): State<AppState>,
    Path((collection_name, id)): Path<(String, String)>,
    Query(params): Query<SingleParams>,
    headers: axum::http::HeaderMap,
) -> Result<Response, (StatusCode, Json<Value>)> {
    if id.eq_ignore_ascii_case("all") {
        return all(
            State(state),
            Path(collection_name),
            Query(AllParams {
                with: params.with,
                expand: params.expand,
                auto_load: params.auto_load,
                limit: params.limit,
                fields: params.fields,
                select: params.select,
                filter: params.filter,
                search: params.search,
                q: params.q,
                sort: params.sort,
                ..Default::default()
            }),
            headers,
        )
        .await
        .map(|r| r.into_response());
    }
    if id.eq_ignore_ascii_case("count") {
        return count(
            State(state),
            Path(collection_name),
            Query(AllParams {
                filter: params.filter,
                search: params.search,
                q: params.q,
                ..Default::default()
            }),
            headers,
        )
        .await
        .map(|r| r.into_response());
    }
    if id.eq_ignore_ascii_case("first") {
        return first(
            State(state),
            Path(collection_name),
            Query(AllParams {
                with: params.with,
                expand: params.expand,
                auto_load: params.auto_load,
                fields: params.fields,
                select: params.select,
                filter: params.filter,
                search: params.search,
                q: params.q,
                sort: params.sort,
                ..Default::default()
            }),
            headers,
        )
        .await;
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
    let fields_vec = parse_fields_param(params.fields.as_deref(), params.select.as_deref());
    let fields_refs: Option<Vec<&str>> = fields_vec.as_ref().map(|v| v.iter().map(String::as_str).collect());

    let mut record = get_record_with(&state.db, &col, &id, with_refs.as_deref())
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

    if let Some(ref f_refs) = fields_refs {
        project_record_fields(&mut record, Some(f_refs));
    }

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

    let (mut payload, pending_files) = parse_request_payload(req, &state).await?;
    validate_role_if_present(&state.db, &payload).await?;

    let auth_user_id = auth.as_ref().map(|c| c.sub.clone());
    if col.blameable || col.collection_type == crate::core::schema::CollectionType::Auth {
        if let Some(ref uid) = auth_user_id {
            if !payload.contains_key("created_by") {
                payload.insert("created_by".to_string(), Value::String(uid.clone()));
            }
            if !payload.contains_key("updated_by") {
                payload.insert("updated_by".to_string(), Value::String(uid.clone()));
            }
        }
    }

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
        state.realtime.broadcast(RealtimeMessage::new(
            "create",
            &col.name,
            serde_json::to_value(&record).unwrap_or_default(),
        ));
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
    validate_role_if_present(&state.db, &payload).await?;

    // Save any uploaded files directly to storage for this record
    if !pending_files.is_empty() {
        let rec_dir = state.storage_dir.join(&col.name).join(&id);
        let _ = tokio::fs::create_dir_all(&rec_dir).await;

        for (field_name, safe_name, data) in pending_files {
            // Delete old file if present in existing record for this field to override cleanly
            if let Some(old_val) = existing.data.get(&field_name).and_then(|v| v.as_str()) {
                if !old_val.is_empty() && old_val != safe_name {
                    let old_dest = rec_dir.join(old_val);
                    if old_dest.exists() {
                        let _ = tokio::fs::remove_file(&old_dest).await;
                    }
                }
            }

            let dest = rec_dir.join(&safe_name);
            let _ = tokio::fs::write(&dest, &data).await;
            payload.insert(field_name, Value::String(safe_name));
        }
    }

    // Check if any file field was explicitly set to null/empty string to remove the file
    for field in &col.schema {
        if field.field_type == crate::core::schema::FieldType::File {
            if let Some(new_val) = payload.get(&field.name) {
                if new_val.is_null() || new_val.as_str() == Some("") {
                    if let Some(old_val) = existing.data.get(&field.name).and_then(|v| v.as_str()) {
                        if !old_val.is_empty() {
                            let rec_dir = state.storage_dir.join(&col.name).join(&id);
                            let old_dest = rec_dir.join(old_val);
                            if old_dest.exists() {
                                let _ = tokio::fs::remove_file(&old_dest).await;
                            }
                        }
                    }
                }
            }
        }
    }

    let auth_user_id = auth.as_ref().map(|c| c.sub.clone());
    if col.blameable || col.collection_type == crate::core::schema::CollectionType::Auth {
        if let Some(ref uid) = auth_user_id {
            payload.insert("updated_by".to_string(), Value::String(uid.clone()));
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
        state.realtime.broadcast(RealtimeMessage::new(
            "update",
            &col.name,
            serde_json::to_value(&record).unwrap_or_default(),
        ));
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
        // Remove storage directory for this record to clean up all uploaded files
        let rec_dir = state.storage_dir.join(&col.name).join(&id);
        if rec_dir.exists() {
            let _ = tokio::fs::remove_dir_all(&rec_dir).await;
        }

        // Realtime broadcast event (only if enabled on this collection)
        if col.realtime {
            state.realtime.broadcast(RealtimeMessage::new(
                "delete",
                &col.name,
                serde_json::to_value(&existing).unwrap_or_default(),
            ));
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
                    state.realtime.broadcast(RealtimeMessage::new(
                        "create",
                        &col.name,
                        serde_json::to_value(&rec).unwrap_or_default(),
                    ));
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

#[derive(Deserialize)]
#[serde(untagged)]
pub enum BatchIdsPayload {
    Object { ids: Vec< Value > },
    Array(Vec<Value>),
}

/// Delete multiple records in a single batch (`POST /api/{collection}/delete-batch` or `POST /api/{collection}/batch-delete`).
pub async fn delete_batch(
    State(state): State<AppState>,
    Path(collection_name): Path<String>,
    headers: axum::http::HeaderMap,
    Json(payload): Json<BatchIdsPayload>,
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
    let raw_ids = match payload {
        BatchIdsPayload::Object { ids } => ids,
        BatchIdsPayload::Array(ids) => ids,
    };

    let ids: Vec<String> = raw_ids
        .into_iter()
        .filter_map(|v| match v {
            Value::String(s) if !s.trim().is_empty() => Some(s.trim().to_string()),
            Value::Number(n) => Some(n.to_string()),
            _ => None,
        })
        .collect();

    if ids.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({ "code": 400, "message": "No record IDs provided for batch deletion." })),
        ));
    }

    // Check delete permission on each record when ownership rule is active
    let mut authorized_ids = Vec::with_capacity(ids.len());
    for id in &ids {
        if let Ok(Some(existing)) = get_record(&state.db, &col, id).await {
            let existing_map = serde_json::to_value(&existing)
                .ok()
                .and_then(|v| v.as_object().cloned());
            check_permission(
                RecordAction::Delete,
                &col,
                auth.as_ref(),
                existing_map.as_ref(),
            )?;
            authorized_ids.push((id.clone(), existing));
        } else {
            // Also verify general collection-level delete permission if record doesn't exist
            check_permission(RecordAction::Delete, &col, auth.as_ref(), None)?;
        }
    }

    let id_strings: Vec<String> = authorized_ids.iter().map(|(id, _)| id.clone()).collect();
    let deleted_count = delete_records_batch(&state.db, &col, &id_strings)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "code": 500, "message": e.to_string() })),
            )
        })?;

    for (id, existing) in authorized_ids {
        let rec_dir = state.storage_dir.join(&col.name).join(&id);
        if rec_dir.exists() {
            let _ = tokio::fs::remove_dir_all(&rec_dir).await;
        }
        if col.realtime {
            state.realtime.broadcast(RealtimeMessage::new(
                "delete",
                &col.name,
                serde_json::to_value(&existing).unwrap_or_default(),
            ));
        }
    }

    Ok(Json(json!({
        "deleted": deleted_count,
        "ids": id_strings,
        "message": format!("Successfully deleted {} records", deleted_count)
    })))
}

#[derive(Deserialize)]
#[serde(untagged)]
pub enum BatchCreatePayload {
    Object { items: Vec<Map<String, Value>> },
    Array(Vec<Map<String, Value>>),
}

/// Create multiple records in a single batch (`POST /api/{collection}/create-batch` or `POST /api/{collection}/batch-create`).
pub async fn create_batch(
    State(state): State<AppState>,
    Path(collection_name): Path<String>,
    headers: axum::http::HeaderMap,
    Json(payload): Json<BatchCreatePayload>,
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

    let items = match payload {
        BatchCreatePayload::Object { items } => items,
        BatchCreatePayload::Array(items) => items,
    };

    if items.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({ "code": 400, "message": "No records provided for batch creation." })),
        ));
    }

    let auth_user_id = auth.as_ref().map(|c| c.sub.clone());
    let mut created_records = Vec::with_capacity(items.len());

    for mut item in items {
        validate_role_if_present(&state.db, &item).await?;
        if col.blameable || col.collection_type == crate::core::schema::CollectionType::Auth {
            if let Some(ref uid) = auth_user_id {
                if !item.contains_key("created_by") {
                    item.insert("created_by".to_string(), Value::String(uid.clone()));
                }
                if !item.contains_key("updated_by") {
                    item.insert("updated_by".to_string(), Value::String(uid.clone()));
                }
            }
        }
        let rec = create_record(&state.db, &col, item).await.map_err(|e| {
            (
                StatusCode::BAD_REQUEST,
                Json(json!({ "code": 400, "message": format!("Failed to create record in batch: {}", e) })),
            )
        })?;
        if col.realtime {
            state.realtime.broadcast(RealtimeMessage::new(
                "create",
                &col.name,
                serde_json::to_value(&rec).unwrap_or_default(),
            ));
        }
        created_records.push(rec);
    }

    Ok((
        StatusCode::CREATED,
        Json(json!({
            "created": created_records.len(),
            "items": created_records
        })),
    ))
}
