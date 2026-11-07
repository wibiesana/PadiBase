use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
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
            "dashboard_ui": "/_padibase/"
        }
    }))
}

pub async fn health_check(State(state): State<AppState>) -> impl IntoResponse {
    let db_ok = state.db.fetch_optional("SELECT 1", &[]).await.is_ok();
    let status = if db_ok {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };
    (
        status,
        Json(json!({
            "code": status.as_u16(),
            "message": if db_ok { "padiBase is healthy and running" } else { "Database connection check failed" },
            "data": {
                "version": env!("CARGO_PKG_VERSION"),
                "engine": "rust",
                "database": state.db.driver_name(),
                "database_connected": db_ok,
                "supported_databases": ["sqlite", "mysql", "mariadb", "postgres"]
            }
        })),
    )
}
