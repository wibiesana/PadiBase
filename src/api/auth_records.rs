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
    authenticate_headers, burn_dummy_verify, check_permission, check_session, create_jwt_with_key,
    generate_token_key, hash_password, verify_password, RecordAction,
};
use crate::core::db::{get_collection_by_name_or_id, get_table_metadata, json_get_str, Val};
use crate::core::env::Env;
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
    let row = match state
        .db
        .fetch_optional(&sql, &[Val::from(login_id), Val::from(login_id)])
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "code": 500, "message": e.to_string() })),
            )
        })? {
        Some(r) => r,
        None => {
            burn_dummy_verify(&payload.password);
            return Err((
                StatusCode::BAD_REQUEST,
                Json(json!({ "code": 400, "message": "Invalid credentials." })),
            ));
        }
    };

    let id = json_get_str(&row, "id");
    let email = json_get_str(&row, "email");
    let status = json_get_str(&row, "status");
    if !status.trim().is_empty()
        && !status.eq_ignore_ascii_case("active")
        && !status.eq_ignore_ascii_case("1")
        && !status.eq_ignore_ascii_case("true")
    {
        return Err((
            StatusCode::FORBIDDEN,
            Json(json!({
                "code": 403,
                "message": format!("Account is currently '{}' and cannot sign in.", status.trim())
            })),
        ));
    }
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

    let mut token_key = json_get_str(&row, "tokenKey");
    if token_key.is_empty() {
        if let Ok(meta) = get_table_metadata(&state.db, &col.name).await {
            if meta.has_column("tokenKey") {
                let generated = generate_token_key();
                let tk_sql = format!(
                    "UPDATE `{}` SET `tokenKey` = ? WHERE `{}` = ?;",
                    col.name, meta.pk_column
                );
                if state
                    .db
                    .execute(&tk_sql, &[Val::from(&generated), Val::from(&id)])
                    .await
                    .is_ok()
                {
                    token_key = generated;
                }
            }
        }
    }

    let token = create_jwt_with_key(
        &id,
        &email,
        &role,
        Some(col.id.clone()),
        14,
        Some(&token_key),
    )
    .map_err(|e| {
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

    // Prevent non-admin users from escalating their own role or forging internal auth columns during public registration
    let is_admin = auth.as_ref().map(|c| c.is_admin()).unwrap_or(false);
    let initial_token_key = generate_token_key();
    data.insert(
        "tokenKey".to_string(),
        Value::String(initial_token_key.clone()),
    );
    data.remove("passwordHash");
    data.remove("remember_token");
    data.remove("last_login_at");
    if !is_admin {
        data.insert("role".to_string(), Value::String("user".to_string()));
        data.remove("status");
        data.remove("verified");
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

    let token = create_jwt_with_key(
        &record.id,
        &email,
        &role,
        Some(col.id.clone()),
        14,
        Some(&initial_token_key),
    )
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "code": 500, "message": e.to_string() })),
        )
    })?;

    if col.realtime {
        if let Ok(val) = serde_json::to_value(&record) {
            state.realtime.broadcast(RealtimeMessage::new("create", &col.name, val));
        }
    }

    // Asynchronously send welcome email to newly registered user if enabled in settings
    let user_name = record
        .data
        .get("name")
        .or_else(|| record.data.get("username"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let reg_email = email.clone();
    let col_name_str = col.name.clone();
    let db_clone = state.db.clone();
    tokio::spawn(async move {
        let config = crate::core::email::get_smtp_config(&db_clone).await;
        if config.enabled && config.events.welcome {
            let (app_name, app_url) = crate::core::email::get_app_meta(&db_clone).await;
            let (subject, html) = crate::core::email::render_welcome_email(
                config.templates.welcome.as_ref(),
                &user_name,
                &reg_email,
                &col_name_str,
                &app_name,
                &app_url,
            );
            let _ = crate::core::email::send_email_with_config(&config, &reg_email, &subject, &html).await;
        }
    });

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

    // Asynchronously dispatch password reset email containing the token if enabled in settings
    if !email.is_empty() && email.contains('@') {
        let recipient_email = email.clone();
        let token_str = reset_token.clone();
        let db_clone = state.db.clone();
        tokio::spawn(async move {
            let config = crate::core::email::get_smtp_config(&db_clone).await;
            if config.enabled && config.events.forgot_password {
                let (app_name, app_url) = crate::core::email::get_app_meta(&db_clone).await;
                let (subject, html) = crate::core::email::render_forgot_password_email(
                    config.templates.forgot_password.as_ref(),
                    &recipient_email,
                    &token_str,
                    &app_name,
                    &app_url,
                );
                let _ = crate::core::email::send_email_with_config(&config, &recipient_email, &subject, &html).await;
            }
        });
    }

    if Env::is_production() {
        return Ok(Json(json!({
            "message": "If the account exists, password reset instructions have been sent."
        })));
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
    crate::core::auth::invalidate_session_cache(&user_id);

    // Asynchronously send notification that password was successfully reset if enabled in settings
    let notify_email = json_get_str(&row, "email");
    if !notify_email.is_empty() && notify_email.contains('@') {
        let db_clone = state.db.clone();
        tokio::spawn(async move {
            let config = crate::core::email::get_smtp_config(&db_clone).await;
            if config.enabled && config.events.password_changed {
                let (app_name, app_url) = crate::core::email::get_app_meta(&db_clone).await;
                let (subject, html) = crate::core::email::render_password_changed_email(
                    config.templates.password_changed.as_ref(),
                    &notify_email,
                    &app_name,
                    &app_url,
                );
                let _ = crate::core::email::send_email_with_config(&config, &notify_email, &subject, &html).await;
            }
        });
    }

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
    check_session(&state, &headers).await?;

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
    crate::core::auth::invalidate_session_cache(&claims.sub);

    let new_token = create_jwt_with_key(
        &claims.sub,
        &claims.email,
        &claims.role,
        Some(col.id.clone()),
        14,
        Some(&rotated_token_key),
    )
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "code": 500, "message": e.to_string() })),
        )
    })?;

    // Asynchronously send notification that password was updated if enabled in settings
    if !claims.email.is_empty() && claims.email.contains('@') {
        let notify_email = claims.email.clone();
        let db_clone = state.db.clone();
        tokio::spawn(async move {
            let config = crate::core::email::get_smtp_config(&db_clone).await;
            if config.enabled && config.events.password_changed {
                let (app_name, app_url) = crate::core::email::get_app_meta(&db_clone).await;
                let (subject, html) = crate::core::email::render_password_changed_email(
                    config.templates.password_changed.as_ref(),
                    &notify_email,
                    &app_name,
                    &app_url,
                );
                let _ = crate::core::email::send_email_with_config(&config, &notify_email, &subject, &html).await;
            }
        });
    }

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
    check_session(&state, &headers).await?;

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

    let status = record
        .data
        .get("status")
        .and_then(|v| v.as_str())
        .unwrap_or("active");
    if !status.trim().is_empty()
        && !status.eq_ignore_ascii_case("active")
        && !status.eq_ignore_ascii_case("1")
        && !status.eq_ignore_ascii_case("true")
    {
        return Err((
            StatusCode::FORBIDDEN,
            Json(json!({
                "code": 403,
                "message": format!("Account is currently '{}'.", status.trim())
            })),
        ));
    }

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

    let token = create_jwt_with_key(
        &record.id,
        &email,
        &role,
        Some(col.id.clone()),
        14,
        claims.tk.as_deref(),
    )
    .map_err(|e| {
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

/// Logout endpoint: `POST /api/{collection}/logout` (rotates tokenKey & invalidates session)
pub async fn logout(
    State(state): State<AppState>,
    Path(collection_name): Path<String>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, (StatusCode, Json<Value>)> {
    handle_logout(state, Some(collection_name), headers).await
}

/// Shortcut logout endpoint: `POST /api/auth/logout`
pub async fn logout_default(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, (StatusCode, Json<Value>)> {
    handle_logout(state, None, headers).await
}

async fn handle_logout(
    state: AppState,
    collection_override: Option<String>,
    headers: HeaderMap,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    if let Some(claims) = authenticate_headers(&headers)? {
        crate::core::auth::invalidate_session_cache(&claims.sub);
        let col_lookup = collection_override
            .or_else(|| claims.collection_id.clone())
            .unwrap_or_else(|| "users".to_string());

        if let Ok(col) = resolve_auth_collection(&state, &col_lookup).await {
            if let Ok(meta) = get_table_metadata(&state.db, &col.name).await {
                if meta.has_column("tokenKey") {
                    let rotated = generate_token_key();
                    let sql = format!(
                        "UPDATE `{}` SET `tokenKey` = ? WHERE `{}` = ?;",
                        col.name, meta.pk_column
                    );
                    let _ = state
                        .db
                        .execute(&sql, &[Val::from(&rotated), Val::from(&claims.sub)])
                        .await;
                }
            }
        }
    }

    Ok(Json(json!({
        "success": true,
        "message": "Logged out successfully."
    })))
}

/// Update current user's profile: `PATCH / PUT /api/{collection}/me`
pub async fn update_profile(
    State(state): State<AppState>,
    Path(collection_name): Path<String>,
    headers: HeaderMap,
    Json(payload): Json<Map<String, Value>>,
) -> Result<impl IntoResponse, (StatusCode, Json<Value>)> {
    handle_update_profile(state, Some(collection_name), headers, payload).await
}

/// Shortcut update current user's profile: `PATCH / PUT /api/auth/me` or `/api/auth/profile`
pub async fn update_profile_default(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<Map<String, Value>>,
) -> Result<impl IntoResponse, (StatusCode, Json<Value>)> {
    handle_update_profile(state, None, headers, payload).await
}

async fn handle_update_profile(
    state: AppState,
    collection_override: Option<String>,
    headers: HeaderMap,
    mut payload: Map<String, Value>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let claims = authenticate_headers(&headers)?.ok_or_else(|| {
        (
            StatusCode::UNAUTHORIZED,
            Json(json!({ "code": 401, "message": "Authentication required." })),
        )
    })?;
    check_session(&state, &headers).await?;

    let col_lookup = collection_override
        .or_else(|| claims.collection_id.clone())
        .unwrap_or_else(|| "users".to_string());
    let col = resolve_auth_collection(&state, &col_lookup).await?;

    // Prevent non-admin users from escalating their own role, status, or verification state
    if !claims.is_admin() {
        payload.remove("role");
        payload.remove("status");
        payload.remove("verified");
    }
    payload.remove("id");
    payload.remove("tokenKey");
    payload.remove("passwordHash");
    payload.remove("password");
    payload.remove("remember_token");
    payload.remove("last_login_at");

    let record = crate::core::record::update_record(&state.db, &col, &claims.sub, payload)
        .await
        .map_err(|e| {
            (
                StatusCode::BAD_REQUEST,
                Json(json!({ "code": 400, "message": format!("Failed to update profile: {}", e) })),
            )
        })?;

    if col.realtime {
        if let Ok(val) = serde_json::to_value(&record) {
            state.realtime.broadcast(RealtimeMessage::new("update", &col.name, val));
        }
    }

    Ok(Json(json!({
        "record": record,
        "message": "Profile updated successfully."
    })))
}
