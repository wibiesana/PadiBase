use axum::{
    extract::Path,
    middleware,
    routing::{delete, get, post, put},
    Router,
};
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;

use crate::core::app::AppState;
use crate::core::response::padi_response_middleware;
use crate::ui::serve_admin_ui;

use super::{admins, api_collection, auth_records, backups, collections, files, health, realtime, records};

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
        // Admins & Settings
        .route("/api/admins/has-admins", get(admins::has_admins))
        .route("/api/admins/auth-with-password", post(admins::auth_with_password))
        .route("/api/admins", get(admins::list_admins).post(admins::create_admin))
        .route("/api/admins/{id}", delete(admins::delete_admin))
        .route("/api/settings", get(admins::get_settings).patch(admins::update_settings))
        // Collections
        .route("/api/collections", get(collections::list).post(collections::create))
        .route("/api/collections/sync", post(collections::sync).get(collections::sync))
        .route("/api/collections/import", put(collections::import_collections))
        .route(
            "/api/collections/{name_or_id}",
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
        // Auth Collections Login
        .route(
            "/api/collections/{collection}/auth-with-password",
            post(auth_records::auth_with_password),
        )
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
