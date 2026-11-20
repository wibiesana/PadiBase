use axum::{
    body::Body,
    extract::{Path, Query, State},
    http::{header, HeaderMap, HeaderValue, StatusCode},
    response::Response,
    Json,
};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::core::api_collection::{
    build_all_collections_with_custom_json, build_insomnia_v4_json, build_openapi_v3_json,
    build_single_collection_json, build_single_insomnia_v4_json, build_single_openapi_v3_json,
};
use crate::core::app::AppState;
use crate::core::auth::authenticate_with_query_token;
use crate::core::db::{get_all_custom_endpoints, get_collection_by_name_or_id, list_collections};
use crate::core::env::Env;

#[derive(Debug, Deserialize, Default)]
pub struct ApiCollectionQuery {
    pub download: Option<bool>,
    pub base_url: Option<String>,
    pub token: Option<String>,
    pub format: Option<String>,
}

async fn ensure_admin(
    state: &AppState,
    headers: &HeaderMap,
    query_token: Option<&str>,
) -> Result<(), (StatusCode, Json<Value>)> {
    let claims = authenticate_with_query_token(state, headers, query_token)
        .await?
        .ok_or_else(|| {
            (
                StatusCode::UNAUTHORIZED,
                Json(json!({ "code": 401, "message": "Administrator authentication required." })),
            )
        })?;
    if !claims.is_admin() {
        return Err((
            StatusCode::FORBIDDEN,
            Json(json!({ "code": 403, "message": "Only administrators can export API collections." })),
        ));
    }
    Ok(())
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
    ensure_admin(&state, &headers, query.token.as_deref()).await?;
    let cols = list_collections(&state.db).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "code": 500, "message": e.to_string() })),
        )
    })?;

    let custom_eps = get_all_custom_endpoints(&state.db).await.unwrap_or_default();
    let base_url = resolve_base_url(&headers, query.base_url.as_deref());
    let fmt = query
        .format
        .as_deref()
        .map(|s| s.trim().to_lowercase())
        .unwrap_or_default();

    let (collection_json, filename) = match fmt.as_str() {
        "insomnia" => (
            build_insomnia_v4_json(&cols, &custom_eps, &base_url),
            "padi_base_api_insomnia.json",
        ),
        "openapi" | "swagger" => (
            build_openapi_v3_json(&cols, &custom_eps, &base_url),
            "padi_base_api_openapi.json",
        ),
        _ => (
            build_all_collections_with_custom_json(&cols, &custom_eps, &base_url),
            "padi_base_api_collection.json",
        ),
    };

    let json_bytes = serde_json::to_vec_pretty(&collection_json).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "code": 500, "message": e.to_string() })),
        )
    })?;

    let mut builder = Response::builder()
        .header(header::CONTENT_TYPE, HeaderValue::from_static("application/json; charset=utf-8"));

    if query.download.unwrap_or(false) {
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
    ensure_admin(&state, &headers, query.token.as_deref()).await?;
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
    let fmt = query
        .format
        .as_deref()
        .map(|s| s.trim().to_lowercase())
        .unwrap_or_default();

    let (collection_json, filename) = match fmt.as_str() {
        "insomnia" => (
            build_single_insomnia_v4_json(&col, &base_url),
            format!("{}_insomnia.json", col.name),
        ),
        "openapi" | "swagger" => (
            build_single_openapi_v3_json(&col, &base_url),
            format!("{}_openapi.json", col.name),
        ),
        _ => (
            build_single_collection_json(&col, &base_url),
            format!("{}_collection.json", col.name),
        ),
    };

    let json_bytes = serde_json::to_vec_pretty(&collection_json).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "code": 500, "message": e.to_string() })),
        )
    })?;

    let mut builder = Response::builder()
        .header(header::CONTENT_TYPE, HeaderValue::from_static("application/json; charset=utf-8"));

    if query.download.unwrap_or(false) {
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
