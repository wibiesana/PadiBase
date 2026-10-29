use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::Deserialize;
use serde_json::json;

use crate::core::app::AppState;
use crate::core::auth::{create_jwt, verify_password};
use crate::core::db::{get_collection_by_name_or_id, json_get_str, Val};
use crate::core::record::get_record;
use crate::core::schema::CollectionType;

#[derive(Deserialize)]
pub struct AuthRecordPasswordRequest {
    pub identity: String,
    pub password: String,
}

pub async fn auth_with_password(
    State(state): State<AppState>,
    Path(collection_name): Path<String>,
    Json(payload): Json<AuthRecordPasswordRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
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

    if col.collection_type != CollectionType::Auth {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({ "code": 400, "message": "Collection is not an auth collection." })),
        ));
    }

    // Query record dari tabel auth (bisa login dengan email atau username)
    let sql = format!(
        "SELECT `id`, `email`, `passwordHash`, `password` FROM `{}` WHERE `email` = ? OR `username` = ? LIMIT 1;",
        col.name
    );
    let row = state
        .db
        .fetch_optional(
            &sql,
            &[Val::from(&payload.identity), Val::from(&payload.identity)],
        )
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "code": 500, "message": e.to_string() })),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::BAD_REQUEST,
                Json(json!({ "code": 400, "message": "Invalid credentials." })),
            )
        })?;

    let id = json_get_str(&row, "id");
    let email = json_get_str(&row, "email");
    let password_hash = {
        let h = json_get_str(&row, "passwordHash");
        if !h.is_empty() {
            h
        } else {
            json_get_str(&row, "password")
        }
    };

    if !verify_password(&payload.password, &password_hash) {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({ "code": 400, "message": "Invalid credentials." })),
        ));
    }

    // Update last_login_at timestamp
    let now_iso = chrono::Utc::now().to_rfc3339();
    let update_login_sql = format!("UPDATE `{}` SET `last_login_at` = ? WHERE `id` = ?;", col.name);
    let _ = state.db.execute(&update_login_sql, &[Val::from(&now_iso), Val::from(&id)]).await;

    let token = create_jwt(&id, &email, "user", Some(col.id.clone()), 14).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "code": 500, "message": e.to_string() })),
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
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "code": 500, "message": "Failed to load record." })),
            )
        })?;

    Ok(Json(json!({
        "token": token,
        "record": record
    })))
}
