use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::core::app::AppState;
use crate::core::auth::{create_jwt, verify_password};
use crate::core::db::{get_system_settings, json_get_i64, json_get_str, save_system_settings, Val};

#[derive(Deserialize)]
pub struct AuthWithPasswordRequest {
    pub identity: String,
    pub password: String,
}

#[derive(Serialize)]
pub struct AdminResponse {
    pub id: String,
    pub email: String,
    pub created: String,
    pub updated: String,
}

pub async fn auth_with_password(
    State(state): State<AppState>,
    Json(payload): Json<AuthWithPasswordRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    let admin = state
        .get_admin_by_email(&payload.identity)
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
                Json(json!({ "code": 400, "message": "Failed to authenticate." })),
            )
        })?;

    if !verify_password(&payload.password, &admin.password_hash) {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({ "code": 400, "message": "Failed to authenticate." })),
        ));
    }

    let token = create_jwt(&admin.id, &admin.email, "admin", None, 7).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "code": 500, "message": e.to_string() })),
        )
    })?;

    Ok(Json(json!({
        "token": token,
        "admin": AdminResponse {
            id: admin.id,
            email: admin.email,
            created: admin.created,
            updated: admin.updated,
        }
    })))
}

pub async fn has_admins(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    let has = state.has_admins().await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "code": 500, "message": e.to_string() })),
        )
    })?;

    Ok(Json(json!({ "hasAdmins": has })))
}

pub async fn list_admins(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    let rows = state
        .db
        .fetch_all(
            "SELECT `id`, `email`, `created`, `updated` FROM `_admins` ORDER BY `created` DESC",
            &[],
        )
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "code": 500, "message": e.to_string() })),
            )
        })?;

    let admins: Vec<AdminResponse> = rows
        .into_iter()
        .map(|r| AdminResponse {
            id: json_get_str(&r, "id"),
            email: json_get_str(&r, "email"),
            created: json_get_str(&r, "created"),
            updated: json_get_str(&r, "updated"),
        })
        .collect();

    Ok(Json(json!({
        "items": admins,
        "totalItems": admins.len()
    })))
}

#[derive(Deserialize)]
pub struct CreateAdminRequest {
    pub email: String,
    pub password: String,
}

pub async fn create_admin(
    State(state): State<AppState>,
    Json(payload): Json<CreateAdminRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    if payload.password.len() < 8 {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({ "code": 400, "message": "Password must be at least 8 characters long." })),
        ));
    }

    let admin = state
        .create_admin(&payload.email, &payload.password)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "code": 500, "message": e.to_string() })),
            )
        })?;

    Ok((
        StatusCode::CREATED,
        Json(AdminResponse {
            id: admin.id,
            email: admin.email,
            created: admin.created,
            updated: admin.updated,
        }),
    ))
}

pub async fn delete_admin(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    let count_row = state
        .db
        .fetch_one("SELECT COUNT(*) as `total` FROM `_admins`", &[])
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "code": 500, "message": e.to_string() })),
            )
        })?;
    let total = json_get_i64(&count_row, "total");
    if total <= 1 {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({ "code": 400, "message": "Cannot delete the last remaining superuser account." })),
        ));
    }

    state
        .db
        .execute("DELETE FROM `_admins` WHERE `id` = ?", &[Val::from(&id)])
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "code": 500, "message": e.to_string() })),
            )
        })?;

    Ok(StatusCode::NO_CONTENT)
}

pub async fn get_settings(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    let settings = get_system_settings(&state.db).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "code": 500, "message": e.to_string() })),
        )
    })?;

    Ok(Json(settings))
}

pub async fn update_settings(
    State(state): State<AppState>,
    Json(payload): Json<serde_json::Value>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    save_system_settings(&state.db, payload.clone()).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "code": 500, "message": e.to_string() })),
        )
    })?;

    Ok(Json(payload))
}
