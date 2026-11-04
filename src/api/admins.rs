use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::core::app::AppState;
use crate::core::auth::{burn_dummy_verify, create_jwt_with_key, require_admin, verify_password};
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
    let admin = match state
        .get_admin_by_email(payload.identity.trim())
        .await
        .map_err(|e| {
            tracing::error!("admin lookup failed: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "code": 500, "message": "Internal server error." })),
            )
        })? {
        Some(a) => a,
        None => {
            // Equalise timing with the "wrong password" path (anti user-enumeration)
            burn_dummy_verify(&payload.password);
            return Err((
                StatusCode::BAD_REQUEST,
                Json(json!({ "code": 400, "message": "Failed to authenticate." })),
            ));
        }
    };

    if !verify_password(&payload.password, &admin.password_hash) {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({ "code": 400, "message": "Failed to authenticate." })),
        ));
    }

    let token = create_jwt_with_key(
        &admin.id,
        &admin.email,
        "admin",
        None,
        7,
        Some(&admin.token_key),
    )
    .map_err(|e| {
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
    headers: HeaderMap,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    require_admin(&state, &headers).await?;
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
    headers: HeaderMap,
    Json(payload): Json<CreateAdminRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    // First-run bootstrap: the very first admin may be created anonymously.
    // Once any admin exists, a valid superuser token is mandatory.
    let has_any = state.has_admins().await.map_err(|e| {
        tracing::error!("has_admins failed: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "code": 500, "message": "Internal server error." })),
        )
    })?;
    if has_any {
        require_admin(&state, &headers).await?;
    }
    let email_ok = {
        let e = payload.email.trim();
        e.len() <= 254 && e.contains('@') && !e.contains(char::is_whitespace)
    };
    if !email_ok {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({ "code": 400, "message": "A valid email address is required." })),
        ));
    }
    if payload.password.len() < 8 || payload.password.len() > 1024 {
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

    // Asynchronously notify newly registered administrator via email if enabled in settings
    let db_clone = state.db.clone();
    let admin_email = admin.email.clone();
    tokio::spawn(async move {
        let config = crate::core::email::get_smtp_config(&db_clone).await;
        if config.enabled && config.events.admin_created {
            let (app_name, app_url) = crate::core::email::get_app_meta(&db_clone).await;
            let (subject, html) = crate::core::email::render_admin_created_email(
                config.templates.admin_created.as_ref(),
                &admin_email,
                &app_name,
                &app_url,
            );
            let _ = crate::core::email::send_email_with_config(&config, &admin_email, &subject, &html).await;
        }
    });

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
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    require_admin(&state, &headers).await?;
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
    headers: HeaderMap,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    let settings = get_system_settings(&state.db).await.map_err(|e| {
        tracing::error!("load settings failed: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "code": 500, "message": "Internal server error." })),
        )
    })?;

    // Superusers get everything (SMTP credentials included); anonymous callers only
    // receive non-sensitive public metadata.
    if require_admin(&state, &headers).await.is_ok() {
        return Ok(Json(settings));
    }
    Ok(Json(json!({
        "meta": settings.get("meta").cloned().unwrap_or(json!({})),
        "roles": settings.get("roles").cloned().unwrap_or(json!([])),
    })))
}

pub async fn update_settings(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<serde_json::Value>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    require_admin(&state, &headers).await?;
    let updated = save_system_settings(&state.db, payload).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "code": 500, "message": e.to_string() })),
        )
    })?;

    Ok(Json(updated))
}

#[derive(Deserialize)]
pub struct TestEmailRequest {
    pub email: String,
    #[serde(default)]
    pub template: Option<String>,
}

pub async fn test_email(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<TestEmailRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    require_admin(&state, &headers).await?;
    let to = payload.email.trim();
    if to.is_empty() || !to.contains('@') {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({ "code": 400, "message": "A valid recipient email address is required." })),
        ));
    }

    let config = crate::core::email::get_smtp_config(&state.db).await;
    if !config.enabled {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({ "code": 400, "message": "SMTP is currently disabled. Please enable it in Mail Settings first." })),
        ));
    }
    if config.host.trim().is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({ "code": 400, "message": "SMTP host is not configured." })),
        ));
    }

    let (app_name, app_url) = crate::core::email::get_app_meta(&state.db).await;
    let template_choice = payload.template.as_deref().unwrap_or("test").to_lowercase();

    let (subject, html_body) = match template_choice.as_str() {
        "welcome" => crate::core::email::render_welcome_email(
            config.templates.welcome.as_ref(),
            "Sample User",
            to,
            "users",
            &app_name,
            &app_url,
        ),
        "forgot_password" | "forgot" => crate::core::email::render_forgot_password_email(
            config.templates.forgot_password.as_ref(),
            to,
            "tkn_demo_987654321",
            &app_name,
            &app_url,
        ),
        "password_changed" | "reset" => crate::core::email::render_password_changed_email(
            config.templates.password_changed.as_ref(),
            to,
            &app_name,
            &app_url,
        ),
        "admin_created" | "admin" => crate::core::email::render_admin_created_email(
            config.templates.admin_created.as_ref(),
            to,
            &app_name,
            &app_url,
        ),
        _ => crate::core::email::render_test_email(to, &app_name, &app_url),
    };

    crate::core::email::send_email_with_config(&config, to, &subject, &html_body)
        .await
        .map_err(|e| {
            (
                StatusCode::BAD_REQUEST,
                Json(json!({
                    "code": 400,
                    "message": format!("Failed to send test email: {}", e)
                })),
            )
        })?;

    Ok(Json(json!({
        "success": true,
        "message": format!("Test email ('{}') successfully sent to {}", template_choice, to)
    })))
}

