use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    Json,
};
use serde::Deserialize;
use serde_json::{json, Map, Value};

use crate::core::app::AppState;
use crate::core::auth::{
    authenticate_headers, check_permission, create_jwt, generate_token_key, hash_password,
    verify_password, RecordAction,
};
use crate::core::db::{get_collection_by_name_or_id, get_table_metadata, json_get_str, Val};
use crate::core::realtime::RealtimeMessage;
use crate::core::record::{create_record, get_record};
use crate::core::schema::{Collection, CollectionType};

#[derive(Deserialize)]
pub struct AuthRecordPasswordRequest {
    #[serde(default)]
    pub identity: String,
    #[serde(default)]
    pub email: String,
    #[serde(default)]
    pub username: String,
    pub password: String,
    #[serde(default)]
    pub collection: Option<String>,
}

#[derive(Deserialize)]
pub struct RegisterRecordRequest {
    pub email: String,
    pub password: String,
    #[serde(default, alias = "passwordConfirm")]
    pub password_confirm: Option<String>,
    #[serde(default)]
    pub username: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub collection: Option<String>,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

#[derive(Deserialize)]
pub struct ForgotPasswordRequest {
    #[serde(default)]
    pub email: String,
    #[serde(default)]
    pub identity: String,
    #[serde(default)]
    pub collection: Option<String>,
}

#[derive(Deserialize)]
pub struct ResetPasswordRequest {
    pub token: String,
    #[serde(default, alias = "newPassword", alias = "new_password")]
    pub password: String,
    #[serde(default, alias = "passwordConfirm")]
    pub password_confirm: Option<String>,
    #[serde(default)]
    pub collection: Option<String>,
}

#[derive(Deserialize)]
pub struct ChangePasswordRequest {
    #[serde(alias = "oldPassword", alias = "current_password", alias = "currentPassword")]
    pub old_password: String,
    #[serde(alias = "newPassword", alias = "password")]
    pub new_password: String,
    #[serde(default, alias = "passwordConfirm")]
    pub password_confirm: Option<String>,
    #[serde(default)]
    pub collection: Option<String>,
}

async fn resolve_auth_collection(
    state: &AppState,
    collection_name: &str,
) -> Result<Collection, (StatusCode, Json<Value>)> {
    let col = get_collection_by_name_or_id(&state.db, collection_name)
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

    Ok(col)
}

/// Login endpoint: `POST /api/collections/{collection}/auth-with-password`
pub async fn auth_with_password(
    State(state): State<AppState>,
    Path(collection_name): Path<String>,
    Json(payload): Json<AuthRecordPasswordRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<Value>)> {
    handle_auth_with_password(state, &collection_name, payload).await
}

/// Shortcut login endpoint: `POST /api/auth/login` (defaults to `users` collection)
pub async fn auth_with_password_default(
    State(state): State<AppState>,
    Json(payload): Json<AuthRecordPasswordRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<Value>)> {
    let col_name = payload
        .collection
        .clone()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| "users".to_string());
    handle_auth_with_password(state, &col_name, payload).await
}

async fn handle_auth_with_password(
    state: AppState,
    collection_name: &str,
    payload: AuthRecordPasswordRequest,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let col = resolve_auth_collection(&state, collection_name).await?;

    let login_id = if !payload.identity.trim().is_empty() {
        payload.identity.trim()
    } else if !payload.email.trim().is_empty() {
        payload.email.trim()
    } else {
        payload.username.trim()
    };

    if login_id.is_empty() || payload.password.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({ "code": 400, "message": "Identity/email and password are required." })),
        ));
    }

    // Query record dari tabel auth (bisa login dengan email atau username)
    let sql = format!(
        "SELECT * FROM `{}` WHERE `email` = ? OR `username` = ? LIMIT 1;",
        col.name
    );
    let row = state
        .db
        .fetch_optional(&sql, &[Val::from(login_id), Val::from(login_id)])
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
    let raw_role = json_get_str(&row, "role");
    let role = if !raw_role.trim().is_empty() {
        raw_role.trim().to_string()
    } else {
        "user".to_string()
    };
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

    // Update last_login_at timestamp if column exists
    let now_iso = chrono::Utc::now().to_rfc3339();
    let update_login_sql = format!(
        "UPDATE `{}` SET `last_login_at` = ? WHERE `id` = ?;",
        col.name
    );
    let _ = state
        .db
        .execute(&update_login_sql, &[Val::from(&now_iso), Val::from(&id)])
        .await;

    let token = create_jwt(&id, &email, &role, Some(col.id.clone()), 14).map_err(|e| {
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

/// Register endpoint: `POST /api/collections/{collection}/register`
pub async fn register(
    State(state): State<AppState>,
    Path(collection_name): Path<String>,
    headers: HeaderMap,
    Json(payload): Json<RegisterRecordRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<Value>)> {
    handle_register(state, &collection_name, headers, payload).await
}

/// Shortcut register endpoint: `POST /api/auth/register` (defaults to `users` collection)
pub async fn register_default(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<RegisterRecordRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<Value>)> {
    let col_name = payload
        .collection
        .clone()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| "users".to_string());
    handle_register(state, &col_name, headers, payload).await
}

async fn handle_register(
    state: AppState,
    collection_name: &str,
    headers: HeaderMap,
    payload: RegisterRecordRequest,
) -> Result<(StatusCode, Json<Value>), (StatusCode, Json<Value>)> {
    let col = resolve_auth_collection(&state, collection_name).await?;
    let auth = authenticate_headers(&headers)?;
    check_permission(RecordAction::Create, &col, auth.as_ref(), None)?;

    let email = payload.email.trim().to_string();
    if email.is_empty() || !email.contains('@') {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({ "code": 400, "message": "A valid email address is required." })),
        ));
    }

    if payload.password.len() < 6 {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({ "code": 400, "message": "Password must be at least 6 characters." })),
        ));
    }

    if let Some(ref confirm) = payload.password_confirm {
        if confirm != &payload.password {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(json!({ "code": 400, "message": "Password confirmation does not match." })),
            ));
        }
    }

    // Check if email already exists
    let check_sql = format!("SELECT `id` FROM `{}` WHERE `email` = ? LIMIT 1;", col.name);
    if let Ok(Some(_)) = state
        .db
        .fetch_optional(&check_sql, &[Val::from(&email)])
        .await
    {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({ "code": 400, "message": "Email is already registered." })),
        ));
    }

    // Check if username already exists (when explicitly provided)
    if let Some(ref uname) = payload.username {
        let clean_uname = uname.trim();
        if !clean_uname.is_empty() {
            let check_uname_sql = format!(
                "SELECT `id` FROM `{}` WHERE `username` = ? LIMIT 1;",
                col.name
            );
            if let Ok(Some(_)) = state
                .db
                .fetch_optional(&check_uname_sql, &[Val::from(clean_uname)])
                .await
            {
                return Err((
                    StatusCode::BAD_REQUEST,
                    Json(json!({ "code": 400, "message": "Username is already taken." })),
                ));
            }
        }
    }

    let mut data = payload.extra;
    data.insert("email".to_string(), Value::String(email.clone()));
    data.insert("password".to_string(), Value::String(payload.password));
    if let Some(uname) = payload.username {
        if !uname.trim().is_empty() {
            data.insert("username".to_string(), Value::String(uname.trim().to_string()));
        }
    }
    if let Some(name) = payload.name {
        data.insert("name".to_string(), Value::String(name));
    }

    // Prevent non-admin users from escalating their own role during public registration
    let is_admin = auth
        .as_ref()
        .map(|c| c.role == "admin" || c.role == "superadmin")
        .unwrap_or(false);
    if !is_admin {
        data.insert("role".to_string(), Value::String("user".to_string()));
    } else if let Some(role_str) = data.get("role").and_then(|v| v.as_str()) {
        let trimmed = role_str.trim();
        if !trimmed.is_empty() {
            let valid_roles = crate::core::db::get_available_roles(&state.db).await;
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

    let record = create_record(&state.db, &col, data).await.map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            Json(json!({ "code": 400, "message": e.to_string() })),
        )
    })?;

    let role = record
        .data
        .get("role")
        .and_then(|v| v.as_str())
        .filter(|s| !s.trim().is_empty())
        .unwrap_or("user")
        .to_string();

    let token = create_jwt(&record.id, &email, &role, Some(col.id.clone()), 14).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "code": 500, "message": e.to_string() })),
        )
    })?;

    if col.realtime {
        if let Ok(val) = serde_json::to_value(&record) {
            state.realtime.broadcast(RealtimeMessage {
                action: "create".to_string(),
                collection: col.name.clone(),
                record: val,
            });
        }
    }

    Ok((
        StatusCode::CREATED,
        Json(json!({
            "token": token,
            "record": record
        })),
    ))
}

/// Forgot password endpoint: `POST /api/collections/{collection}/forgot-password`
pub async fn forgot_password(
    State(state): State<AppState>,
    Path(collection_name): Path<String>,
    Json(payload): Json<ForgotPasswordRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<Value>)> {
    handle_forgot_password(state, &collection_name, payload).await
}

/// Shortcut forgot password endpoint: `POST /api/auth/forgot-password` (defaults to `users` collection)
pub async fn forgot_password_default(
    State(state): State<AppState>,
    Json(payload): Json<ForgotPasswordRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<Value>)> {
    let col_name = payload
        .collection
        .clone()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| "users".to_string());
    handle_forgot_password(state, &col_name, payload).await
}

async fn handle_forgot_password(
    state: AppState,
    collection_name: &str,
    payload: ForgotPasswordRequest,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let col = resolve_auth_collection(&state, collection_name).await?;
    let identity = if !payload.email.trim().is_empty() {
        payload.email.trim()
    } else {
        payload.identity.trim()
    };

    if identity.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({ "code": 400, "message": "Email or identity is required." })),
        ));
    }

    let find_sql = format!(
        "SELECT * FROM `{}` WHERE `email` = ? OR `username` = ? LIMIT 1;",
        col.name
    );
    let row = state
        .db
        .fetch_optional(&find_sql, &[Val::from(identity), Val::from(identity)])
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
                Json(json!({ "code": 404, "message": "Account with that email/identity was not found." })),
            )
        })?;

    let user_id = json_get_str(&row, "id");
    let email = json_get_str(&row, "email");
    let reset_token = generate_token_key();
    let now_iso = chrono::Utc::now().to_rfc3339();

    let meta = get_table_metadata(&state.db, &col.name).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "code": 500, "message": e.to_string() })),
        )
    })?;

    let mut set_parts = Vec::new();
    let mut params = Vec::new();

    if meta.has_column("tokenKey") {
        set_parts.push("`tokenKey` = ?");
        params.push(Val::from(&reset_token));
    }
    if meta.has_column("remember_token") {
        set_parts.push("`remember_token` = ?");
        params.push(Val::from(&reset_token));
    }
    if meta.has_updated {
        set_parts.push("`updated` = ?");
        params.push(Val::from(&now_iso));
    }

    if !set_parts.is_empty() {
        params.push(Val::from(&user_id));
        let update_sql = format!(
            "UPDATE `{}` SET {} WHERE `{}` = ?;",
            col.name,
            set_parts.join(", "),
            meta.pk_column
        );
        state.db.execute(&update_sql, &params).await.map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "code": 500, "message": e.to_string() })),
            )
        })?;
    }

    Ok(Json(json!({
        "message": "Password reset token generated successfully.",
        "email": email,
        "reset_token": reset_token
    })))
}

/// Reset password endpoint: `POST /api/collections/{collection}/reset-password`
pub async fn reset_password(
    State(state): State<AppState>,
    Path(collection_name): Path<String>,
    Json(payload): Json<ResetPasswordRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<Value>)> {
    handle_reset_password(state, &collection_name, payload).await
}

/// Shortcut reset password endpoint: `POST /api/auth/reset-password` (defaults to `users` collection)
pub async fn reset_password_default(
    State(state): State<AppState>,
    Json(payload): Json<ResetPasswordRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<Value>)> {
    let col_name = payload
        .collection
        .clone()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| "users".to_string());
    handle_reset_password(state, &col_name, payload).await
}

async fn handle_reset_password(
    state: AppState,
    collection_name: &str,
    payload: ResetPasswordRequest,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let col = resolve_auth_collection(&state, collection_name).await?;

    let token = payload.token.trim();
    if token.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({ "code": 400, "message": "Reset token is required." })),
        ));
    }

    if payload.password.len() < 6 {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({ "code": 400, "message": "New password must be at least 6 characters." })),
        ));
    }

    if let Some(ref confirm) = payload.password_confirm {
        if confirm != &payload.password {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(json!({ "code": 400, "message": "Password confirmation does not match." })),
            ));
        }
    }

    let meta = get_table_metadata(&state.db, &col.name).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "code": 500, "message": e.to_string() })),
        )
    })?;

    let find_sql = if meta.has_column("remember_token") && meta.has_column("tokenKey") {
        format!(
            "SELECT * FROM `{}` WHERE `tokenKey` = ? OR `remember_token` = ? LIMIT 1;",
            col.name
        )
    } else if meta.has_column("remember_token") {
        format!(
            "SELECT * FROM `{}` WHERE `remember_token` = ? LIMIT 1;",
            col.name
        )
    } else {
        format!("SELECT * FROM `{}` WHERE `tokenKey` = ? LIMIT 1;", col.name)
    };

    let find_params = if meta.has_column("remember_token") && meta.has_column("tokenKey") {
        vec![Val::from(token), Val::from(token)]
    } else {
        vec![Val::from(token)]
    };

    let row = state
        .db
        .fetch_optional(&find_sql, &find_params)
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
                Json(json!({ "code": 400, "message": "Invalid or expired password reset token." })),
            )
        })?;

    let user_id = json_get_str(&row, &meta.pk_column);
    let new_hash = hash_password(&payload.password).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "code": 500, "message": e.to_string() })),
        )
    })?;
    let rotated_token_key = generate_token_key();
    let now_iso = chrono::Utc::now().to_rfc3339();

    let mut set_parts = Vec::new();
    let mut params = Vec::new();

    if meta.has_column("passwordHash") {
        set_parts.push("`passwordHash` = ?");
        params.push(Val::from(&new_hash));
    }
    if meta.has_column("password") {
        set_parts.push("`password` = ?");
        params.push(Val::from(&new_hash));
    }
    if meta.has_column("tokenKey") {
        set_parts.push("`tokenKey` = ?");
        params.push(Val::from(&rotated_token_key));
    }
    if meta.has_column("remember_token") {
        set_parts.push("`remember_token` = ?");
        params.push(Val::Null);
    }
    if meta.has_updated {
        set_parts.push("`updated` = ?");
        params.push(Val::from(&now_iso));
    }

    params.push(Val::from(&user_id));
    let update_sql = format!(
        "UPDATE `{}` SET {} WHERE `{}` = ?;",
        col.name,
        set_parts.join(", "),
        meta.pk_column
    );
    state.db.execute(&update_sql, &params).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "code": 500, "message": e.to_string() })),
        )
    })?;

    Ok(Json(json!({
        "message": "Password has been reset successfully."
    })))
}

/// Change password for currently logged-in user: `POST /api/collections/{collection}/change-password`
pub async fn change_password(
    State(state): State<AppState>,
    Path(collection_name): Path<String>,
    headers: HeaderMap,
    Json(payload): Json<ChangePasswordRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<Value>)> {
    handle_change_password(state, &collection_name, headers, payload).await
}

/// Shortcut change password endpoint: `POST /api/auth/change-password`
pub async fn change_password_default(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<ChangePasswordRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<Value>)> {
    let col_name = payload
        .collection
        .clone()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| "users".to_string());
    handle_change_password(state, &col_name, headers, payload).await
}

async fn handle_change_password(
    state: AppState,
    collection_name: &str,
    headers: HeaderMap,
    payload: ChangePasswordRequest,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let claims = authenticate_headers(&headers)?.ok_or_else(|| {
        (
            StatusCode::UNAUTHORIZED,
            Json(json!({ "code": 401, "message": "Authentication required." })),
        )
    })?;

    let col = resolve_auth_collection(&state, collection_name).await?;

    if payload.new_password.len() < 6 {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({ "code": 400, "message": "New password must be at least 6 characters." })),
        ));
    }

    if let Some(ref confirm) = payload.password_confirm {
        if confirm != &payload.new_password {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(json!({ "code": 400, "message": "Password confirmation does not match." })),
            ));
        }
    }

    let meta = get_table_metadata(&state.db, &col.name).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "code": 500, "message": e.to_string() })),
        )
    })?;

    let find_sql = format!(
        "SELECT * FROM `{}` WHERE `{}` = ? LIMIT 1;",
        col.name, meta.pk_column
    );
    let row = state
        .db
        .fetch_optional(&find_sql, &[Val::from(&claims.sub)])
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
                Json(json!({ "code": 404, "message": "Authenticated user record not found." })),
            )
        })?;

    let current_hash = {
        let h = json_get_str(&row, "passwordHash");
        if !h.is_empty() {
            h
        } else {
            json_get_str(&row, "password")
        }
    };

    if !verify_password(&payload.old_password, &current_hash) {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({ "code": 400, "message": "Current password is incorrect." })),
        ));
    }

    let new_hash = hash_password(&payload.new_password).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "code": 500, "message": e.to_string() })),
        )
    })?;
    let rotated_token_key = generate_token_key();
    let now_iso = chrono::Utc::now().to_rfc3339();

    let mut set_parts = Vec::new();
    let mut params = Vec::new();

    if meta.has_column("passwordHash") {
        set_parts.push("`passwordHash` = ?");
        params.push(Val::from(&new_hash));
    }
    if meta.has_column("password") {
        set_parts.push("`password` = ?");
        params.push(Val::from(&new_hash));
    }
    if meta.has_column("tokenKey") {
        set_parts.push("`tokenKey` = ?");
        params.push(Val::from(&rotated_token_key));
    }
    if meta.has_updated {
        set_parts.push("`updated` = ?");
        params.push(Val::from(&now_iso));
    }

    params.push(Val::from(&claims.sub));
    let update_sql = format!(
        "UPDATE `{}` SET {} WHERE `{}` = ?;",
        col.name,
        set_parts.join(", "),
        meta.pk_column
    );
    state.db.execute(&update_sql, &params).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "code": 500, "message": e.to_string() })),
        )
    })?;

    let new_token = create_jwt(
        &claims.sub,
        &claims.email,
        &claims.role,
        Some(col.id.clone()),
        14,
    )
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "code": 500, "message": e.to_string() })),
        )
    })?;

    Ok(Json(json!({
        "message": "Password updated successfully.",
        "token": new_token
    })))
}

/// Get current user profile & refresh token: `GET/POST /api/collections/{collection}/auth-refresh` or `/me`
pub async fn auth_refresh(
    State(state): State<AppState>,
    Path(collection_name): Path<String>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, (StatusCode, Json<Value>)> {
    handle_auth_refresh(state, Some(collection_name), headers).await
}

/// Shortcut current user profile & refresh token: `GET /api/auth/me` or `POST /api/auth/refresh`
pub async fn auth_refresh_default(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, (StatusCode, Json<Value>)> {
    handle_auth_refresh(state, None, headers).await
}

async fn handle_auth_refresh(
    state: AppState,
    collection_override: Option<String>,
    headers: HeaderMap,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let claims = authenticate_headers(&headers)?.ok_or_else(|| {
        (
            StatusCode::UNAUTHORIZED,
            Json(json!({ "code": 401, "message": "Authentication required." })),
        )
    })?;

    let col_lookup = collection_override
        .or_else(|| claims.collection_id.clone())
        .unwrap_or_else(|| "users".to_string());

    let col = resolve_auth_collection(&state, &col_lookup).await?;

    let record = get_record(&state.db, &col, &claims.sub)
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
                Json(json!({ "code": 404, "message": "Authenticated user record not found." })),
            )
        })?;

    let email = record
        .data
        .get("email")
        .and_then(|v| v.as_str())
        .unwrap_or(&claims.email)
        .to_string();
    let role = record
        .data
        .get("role")
        .and_then(|v| v.as_str())
        .filter(|s| !s.trim().is_empty())
        .unwrap_or(&claims.role)
        .to_string();

    let token = create_jwt(&record.id, &email, &role, Some(col.id.clone()), 14).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "code": 500, "message": e.to_string() })),
        )
    })?;

    Ok(Json(json!({
        "token": token,
        "record": record
    })))
}
