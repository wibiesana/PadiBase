use axum::{extract::State, response::IntoResponse, Json};
use serde_json::json;

use crate::core::app::AppState;

pub async fn api_root(State(state): State<AppState>) -> impl IntoResponse {
    Json(json!({
        "name": "padiBase",
        "description": "High performance Backend-as-a-Service written in Rust",
        "version": env!("CARGO_PKG_VERSION"),
        "status": "online",
        "database": state.db.driver_name(),
        "supported_databases": ["sqlite", "mysql", "mariadb", "postgres"],
        "endpoints": {
            "health": "/api/health",
            "admins": "/api/admins",
            "has_admins": "/api/admins/has-admins",
            "collections": "/api/collections",
            "tables": "/api/tables",
            "realtime_sse": "/api/realtime",
            "backups": "/api/backups",
            "dashboard_ui": "/dashboard/"
        }
    }))
}

pub async fn health_check(State(state): State<AppState>) -> impl IntoResponse {
    Json(json!({
        "code": 200,
        "message": "padiBase is healthy and running",
        "data": {
            "version": env!("CARGO_PKG_VERSION"),
            "engine": "rust",
            "database": state.db.driver_name(),
            "supported_databases": ["sqlite", "mysql", "mariadb", "postgres"]
        }
    }))
}
