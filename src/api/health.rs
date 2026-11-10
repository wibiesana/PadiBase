use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use serde_json::json;

use crate::core::app::AppState;

pub async fn api_root(State(state): State<AppState>) -> impl IntoResponse {
    let host = crate::core::env::Env::get_or("APP_HOST", "127.0.0.1");
    let port = crate::core::env::Env::get_or("APP_PORT", "8085");
    let base_url = format!("http://{}:{}", host, port);

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
        },
        "js_sdk": {
            "package": "padibase",
            "local_install": "npm install ./padi_js_sdk",
            "quick_setup": format!("import PadiBase from 'padibase';\nexport const padi = new PadiBase('{}');", base_url),
            "examples": {
                "crud": {
                    "list": "const { items, totalItems, page } = await padi.table('users').getList(1, 30, { sort: '-created' });",
                    "all": "const allRecords = await padi.table('users').getFullList({ sort: '-created' });",
                    "get_one": "const record = await padi.table('users').getOne('RECORD_ID');",
                    "create": "const newRecord = await padi.table('users').create({ name: 'John Doe', email: 'john@example.com' });",
                    "update": "const updated = await padi.table('users').update('RECORD_ID', { name: 'Jane Doe' });",
                    "delete": "await padi.table('users').delete('RECORD_ID');"
                },
                "realtime": "const unsubscribe = await padi.table('users').subscribe((e) => {\n  console.log('Realtime Event:', e.action, e.record);\n});\n// To stop: unsubscribe();",
                "files": {
                    "upload": "const formData = new FormData();\nformData.append('avatar', fileInputElement.files[0]);\nconst updated = await padi.files.upload('users', 'RECORD_ID', formData);",
                    "get_url": format!("const fileUrl = padi.files.getUrl('users', 'RECORD_ID', 'avatar.png'); // => {}/api/files/users/RECORD_ID/avatar.png", base_url)
                },
                "auth": {
                    "login": "const authData = await padi.table('users').authWithPassword('john@example.com', 'secret123456');",
                    "register": "const newUser = await padi.table('users').register({ email: 'user@example.com', username: 'user1', password: 'secret123456', passwordConfirm: 'secret123456' });",
                    "profile": "console.log(padi.authStore.record, padi.authStore.isValid);",
                    "logout": "padi.authStore.clear();"
                }
            }
        }
    }))
}

pub async fn health_check(State(state): State<AppState>) -> impl IntoResponse {
    let db_ok = state.db.check_health_cached().await;
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
