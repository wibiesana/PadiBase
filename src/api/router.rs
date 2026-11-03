use axum::{
    extract::Path,
    middleware,
    routing::{any, delete, get, post, put},
    Router,
};
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;

use crate::core::app::AppState;
use crate::core::response::padi_response_middleware;
use crate::ui::serve_admin_ui;

use super::{
    admins, api_collection, auth_records, backups, collections, custom, files, health, realtime,
    records, transactions,
};

pub fn create_router(state: AppState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        // API Root & Health check
        .route("/api", get(health::api_root))
        .route("/api/", get(health::api_root))
        .route("/api/health", get(health::health_check))
        // API Collection Export (Postman / REST Client)
        .route("/api/api-collection", get(api_collection::export_all))
        .route("/api/api-collection.json", get(api_collection::export_all))
        .route("/api/export/api-collection", get(api_collection::export_all))
        .route(
            "/api/collections/{collection}/api-collection",
            get(api_collection::export_single),
        )
        .route(
            "/api/collections/{collection}/export-api-collection",
            get(api_collection::export_single),
        )
        .route(
            "/api/tables/{collection}/api-collection",
            get(api_collection::export_single),
        )
        .route(
            "/api/tables/{collection}/export-api-collection",
            get(api_collection::export_single),
        )
        // Admins & Settings
        .route("/api/admins/has-admins", get(admins::has_admins))
        .route("/api/admins/auth-with-password", post(admins::auth_with_password))
        .route("/api/admins", get(admins::list_admins).post(admins::create_admin))
        .route("/api/admins/{id}", delete(admins::delete_admin))
        .route("/api/settings", get(admins::get_settings).patch(admins::update_settings))
        // Tables & Collections (dual routing: /api/tables and /api/collections)
        .route("/api/collections", get(collections::list).post(collections::create))
        .route("/api/collections/sync", post(collections::sync).get(collections::sync))
        .route("/api/collections/import", put(collections::import_collections))
        .route(
            "/api/collections/{name_or_id}",
            get(collections::get_one)
                .patch(collections::update)
                .delete(collections::delete),
        )
        .route("/api/tables", get(collections::list).post(collections::create))
        .route("/api/tables/sync", post(collections::sync).get(collections::sync))
        .route("/api/tables/import", put(collections::import_collections))
        .route(
            "/api/tables/{name_or_id}",
            get(collections::get_one)
                .patch(collections::update)
                .delete(collections::delete),
        )
        // Records
        .route(
            "/api/collections/{collection}/records",
            get(records::list).post(records::create),
        )
        .route(
            "/api/collections/{collection}/records/all",
            get(records::all),
        )
        .route(
            "/api/collections/{collection}/all",
            get(records::all),
        )
        .route(
            "/api/tables/{collection}/records",
            get(records::list).post(records::create),
        )
        .route(
            "/api/tables/{collection}/records/all",
            get(records::all),
        )
        .route(
            "/api/tables/{collection}/all",
            get(records::all),
        )
        .route(
            "/api/{collection}/all",
            get(records::all),
        )
        .route(
            "/{collection}/all",
            get(records::all),
        )
        .route(
            "/api/collections/{collection}/records/{id}",
            get(records::get_one)
                .patch(records::update)
                .delete(records::delete),
        )
        .route(
            "/api/collections/{collection}/export",
            get(records::export_records),
        )
        .route(
            "/api/collections/{collection}/import",
            post(records::import_records),
        )
        .route(
            "/api/tables/{collection}/records/{id}",
            get(records::get_one)
                .patch(records::update)
                .delete(records::delete),
        )
        .route(
            "/api/tables/{collection}/export",
            get(records::export_records),
        )
        .route(
            "/api/tables/{collection}/import",
            post(records::import_records),
        )
        // Auth Tables & Collections (Login, Register, Forgot Password, Reset Password, Change Password, Me/Refresh)
        .route(
            "/api/collections/{collection}/auth-with-password",
            post(auth_records::auth_with_password),
        )
        .route(
            "/api/collections/{collection}/login",
            post(auth_records::auth_with_password),
        )
        .route(
            "/api/collections/{collection}/register",
            post(auth_records::register),
        )
        .route(
            "/api/collections/{collection}/forgot-password",
            post(auth_records::forgot_password),
        )
        .route(
            "/api/collections/{collection}/request-password-reset",
            post(auth_records::forgot_password),
        )
        .route(
            "/api/collections/{collection}/reset-password",
            post(auth_records::reset_password),
        )
        .route(
            "/api/collections/{collection}/confirm-password-reset",
            post(auth_records::reset_password),
        )
        .route(
            "/api/collections/{collection}/change-password",
            post(auth_records::change_password),
        )
        .route(
            "/api/collections/{collection}/auth-refresh",
            post(auth_records::auth_refresh).get(auth_records::auth_refresh),
        )
        .route(
            "/api/collections/{collection}/me",
            get(auth_records::auth_refresh),
        )
        // Aliases for /api/tables/{table}/...
        .route(
            "/api/tables/{collection}/auth-with-password",
            post(auth_records::auth_with_password),
        )
        .route(
            "/api/tables/{collection}/login",
            post(auth_records::auth_with_password),
        )
        .route(
            "/api/tables/{collection}/register",
            post(auth_records::register),
        )
        .route(
            "/api/tables/{collection}/forgot-password",
            post(auth_records::forgot_password),
        )
        .route(
            "/api/tables/{collection}/request-password-reset",
            post(auth_records::forgot_password),
        )
        .route(
            "/api/tables/{collection}/reset-password",
            post(auth_records::reset_password),
        )
        .route(
            "/api/tables/{collection}/confirm-password-reset",
            post(auth_records::reset_password),
        )
        .route(
            "/api/tables/{collection}/change-password",
            post(auth_records::change_password),
        )
        .route(
            "/api/tables/{collection}/auth-refresh",
            post(auth_records::auth_refresh).get(auth_records::auth_refresh),
        )
        .route(
            "/api/tables/{collection}/me",
            get(auth_records::auth_refresh),
        )
        // Direct Auth Shortcuts (defaults to `users` collection)
        .route("/api/auth/login", post(auth_records::auth_with_password_default))
        .route("/api/auth/register", post(auth_records::register_default))
        .route("/api/auth/forgot-password", post(auth_records::forgot_password_default))
        .route("/api/auth/reset-password", post(auth_records::reset_password_default))
        .route("/api/auth/change-password", post(auth_records::change_password_default))
        .route(
            "/api/auth/me",
            get(auth_records::auth_refresh_default).post(auth_records::auth_refresh_default),
        )
        .route(
            "/api/auth/refresh",
            post(auth_records::auth_refresh_default).get(auth_records::auth_refresh_default),
        )
        // Atomic Database Transactions & Batch Operations
        .route("/api/transactions", post(transactions::execute_transaction))
        .route("/api/batch", post(transactions::execute_transaction))
        // Custom / Raw SQL Query & Execution (Admin Only)
        .route("/api/sql", post(transactions::execute_custom_sql))
        .route("/api/sql/query", post(transactions::execute_custom_sql))
        .route("/api/sql/execute", post(transactions::execute_custom_sql))
        // Dynamic Custom SQL Endpoints Management (Dashboard)
        .route(
            "/api/endpoints",
            get(custom::list_endpoints).post(custom::create_endpoint),
        )
        .route(
            "/api/endpoints/{id_or_path}",
            get(custom::get_endpoint)
                .patch(custom::update_endpoint)
                .put(custom::update_endpoint)
                .delete(custom::delete_endpoint),
        )
        // Dynamic Custom SQL Endpoints Execution & Built-in Custom Endpoints
        .route("/api/custom/{*path}", any(custom::execute_dynamic_endpoint))
        .route("/api/e/{*path}", any(custom::execute_dynamic_endpoint))
        // Realtime SSE
        .route("/api/realtime", get(realtime::realtime_sse))
        // File upload, download & management
        .route(
            "/api/files/{collection}/{record_id}",
            post(files::upload_file),
        )
        .route(
            "/api/files/{collection}/{record_id}/{filename}",
            get(files::get_file).delete(files::delete_file),
        )
        // System Backups
        .route("/api/backups", get(backups::list_backups).post(backups::create_backup))
        .route(
            "/api/backups/{filename}",
            get(backups::download_backup).delete(backups::delete_backup),
        )
        .route("/api/backups/{filename}/restore", post(backups::restore_backup))
        // Admin UI Dashboard
        .route(
            "/dashboard",
            get(|| async { serve_admin_ui(Path(Some("index.html".to_string()))).await }),
        )
        .route(
            "/dashboard/",
            get(|| async { serve_admin_ui(Path(Some("index.html".to_string()))).await }),
        )
        .route(
            "/dashboard/{*path}",
            get(|Path(path): Path<String>| async move { serve_admin_ui(Path(Some(path))).await }),
        )
        .route(
            "/favicon.ico",
            get(|| async { serve_admin_ui(Path(Some("favicon.svg".to_string()))).await }),
        )
        .route(
            "/favicon.svg",
            get(|| async { serve_admin_ui(Path(Some("favicon.svg".to_string()))).await }),
        )
        .route(
            "/",
            get(|| async { serve_admin_ui(Path(Some("index.html".to_string()))).await }),
        )
        .layer(middleware::from_fn_with_state(
            state.clone(),
            padi_response_middleware,
        ))
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
