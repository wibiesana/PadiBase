use axum::{
    body::Body,
    extract::{Path, Query, State},
    http::{header, HeaderMap, HeaderValue, StatusCode},
    response::Response,
    Json,
};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::core::api_collection::{build_all_collections_json, build_single_collection_json};
use crate::core::app::AppState;
use crate::core::db::{get_collection_by_name_or_id, list_collections};
use crate::core::env::Env;

#[derive(Debug, Deserialize, Default)]
pub struct ApiCollectionQuery {
    pub download: Option<bool>,
    pub base_url: Option<String>,
}

fn resolve_base_url(headers: &HeaderMap, query_url: Option<&str>) -> String {
    if let Some(url) = query_url {
        if !url.trim().is_empty() {
            return url.trim().to_string();
        }
    }

    if let Some(host) = headers.get(header::HOST).and_then(|h| h.to_str().ok()) {
        return format!("http://{}", host);
    }

    let host = Env::get_or("APP_HOST", "127.0.0.1");
    let port = Env::get_or("APP_PORT", "8085");
    format!("http://{}:{}", host, port)
}

/// Handler for exporting full API collection (system, admins, backups, and all database collections)
pub async fn export_all(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<ApiCollectionQuery>,
) -> Result<Response, (StatusCode, Json<Value>)> {
    let cols = list_collections(&state.db).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "code": 500, "message": e.to_string() })),
        )
    })?;

    let base_url = resolve_base_url(&headers, query.base_url.as_deref());
    let collection_json = build_all_collections_json(&cols, &base_url);

    let json_bytes = serde_json::to_vec_pretty(&collection_json).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "code": 500, "message": e.to_string() })),
        )
    })?;

    let mut builder = Response::builder()
        .header(header::CONTENT_TYPE, HeaderValue::from_static("application/json; charset=utf-8"));

    if query.download.unwrap_or(false) {
        let filename = "padi_base_api_collection.json";
        let disposition = format!("attachment; filename=\"{}\"", filename);
        builder = builder.header(
            header::CONTENT_DISPOSITION,
            HeaderValue::from_str(&disposition).unwrap_or(HeaderValue::from_static("attachment")),
        );
    }

    builder
        .body(Body::from(json_bytes))
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "code": 500, "message": e.to_string() }))))
}

/// Handler for exporting a single collection's REST API collection
pub async fn export_single(
    State(state): State<AppState>,
    Path(collection_name): Path<String>,
    headers: HeaderMap,
    Query(query): Query<ApiCollectionQuery>,
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
                Json(json!({ "code": 404, "message": format!("Collection '{}' not found.", collection_name) })),
            )
        })?;

    let base_url = resolve_base_url(&headers, query.base_url.as_deref());
    let collection_json = build_single_collection_json(&col, &base_url);

    let json_bytes = serde_json::to_vec_pretty(&collection_json).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "code": 500, "message": e.to_string() })),
        )
    })?;

    let mut builder = Response::builder()
        .header(header::CONTENT_TYPE, HeaderValue::from_static("application/json; charset=utf-8"));

    if query.download.unwrap_or(false) {
        let filename = format!("{}_collection.json", col.name);
        let disposition = format!("attachment; filename=\"{}\"", filename);
        builder = builder.header(
            header::CONTENT_DISPOSITION,
            HeaderValue::from_str(&disposition).unwrap_or(HeaderValue::from_static("attachment")),
        );
    }

    builder
        .body(Body::from(json_bytes))
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "code": 500, "message": e.to_string() }))))
}
