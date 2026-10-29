use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use padi_baas::{api::router::create_router, AppState};
use serde_json::json;
use tower::ServiceExt;

#[tokio::test]
async fn test_full_padi_baas_flow() {
    let test_dir = std::path::PathBuf::from(format!("./target/test_data_{}", uuid::Uuid::new_v4().simple()));
    let _ = std::fs::remove_dir_all(&test_dir);
    let state = AppState::new(&test_dir).await.expect("init app state");
    let app = create_router(state);

    // 1. API Root & Health check
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);

    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);

    // 2. Cek has-admins (harus false diawal)
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/admins/has-admins")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);

    // 3. Buat admin pertama
    let req_body = serde_json::to_vec(&json!({
        "email": "master@padi.local",
        "password": "supersecurepassword123"
    }))
    .unwrap();

    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/admins")
                .header("Content-Type", "application/json")
                .body(Body::from(req_body))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::CREATED);

    // 4. Login admin
    let login_body = serde_json::to_vec(&json!({
        "identity": "master@padi.local",
        "password": "supersecurepassword123"
    }))
    .unwrap();

    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/admins/auth-with-password")
                .header("Content-Type", "application/json")
                .body(Body::from(login_body))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);

    // 5. Buat koleksi dinamis "posts"
    let col_body = serde_json::to_vec(&json!({
        "name": "posts",
        "type": "base",
        "schema": [
            { "name": "title", "type": "text", "required": true },
            { "name": "content", "type": "text" },
            { "name": "views", "type": "number" }
        ]
    }))
    .unwrap();

    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/collections")
                .header("Content-Type", "application/json")
                .body(Body::from(col_body))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::CREATED);

    // 6. Insert record baru ke koleksi "posts"
    let record_body = serde_json::to_vec(&json!({
        "title": "Rust BaaS is awesome",
        "content": "Building PocketBase alternative in Rust with Axum & SQLite",
        "views": 42
    }))
    .unwrap();

    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/collections/posts/records")
                .header("Content-Type", "application/json")
                .body(Body::from(record_body))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::CREATED);

    // 7. Ambil daftar records & verifikasi format response Padi Rust (success, message, message_code, item, meta, debug)
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/collections/posts/records")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);
    let body_bytes = axum::body::to_bytes(res.into_body(), 1024 * 1024)
        .await
        .unwrap();
    let json_val: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(json_val["success"], true);
    assert_eq!(json_val["message_code"], "SUCCESS");
    assert!(json_val["item"].is_array());
    assert_eq!(json_val["meta"]["total"], 1);
    assert!(json_val.get("debug").is_some());
    assert!(json_val["debug"]["queries"].is_array());

    // Clean up
    let _ = std::fs::remove_dir_all(&test_dir);
}

#[test]
fn test_multi_database_url_and_env_resolution() {
    use padi_baas::core::db::{resolve_db_config, DbDriver};
    use std::path::Path;

    let dir = Path::new("./padi_data");

    // Direct URL overrides for all supported database engines
    let (_, drv) = resolve_db_config(dir, Some("mysql://root:123@localhost:3306/padi"));
    assert_eq!(drv, DbDriver::MySql);

    let (url, drv) = resolve_db_config(dir, Some("mariadb://root:123@localhost:3306/padi"));
    assert_eq!(drv, DbDriver::MariaDb);
    assert!(url.starts_with("mysql://"));

    let (_, drv) = resolve_db_config(dir, Some("postgres://postgres:123@localhost:5432/padi"));
    assert_eq!(drv, DbDriver::Postgres);

    let (_, drv) = resolve_db_config(dir, Some("postgresql://postgres:123@localhost:5432/padi"));
    assert_eq!(drv, DbDriver::Postgres);

    let (_, drv) = resolve_db_config(dir, Some("sqlite://./padi_data/data.db"));
    assert_eq!(drv, DbDriver::Sqlite);
}

#[tokio::test]
async fn test_per_collection_realtime_rule() {
    let test_dir = std::path::PathBuf::from(format!(
        "./target/test_rt_data_{}",
        uuid::Uuid::new_v4().simple()
    ));
    let _ = std::fs::remove_dir_all(&test_dir);
    let state = AppState::new(&test_dir).await.expect("init app state");
    let mut rx = state.realtime.subscribe();
    let app = create_router(state);

    // 1. Create collection with `realtime: false`
    let col_body = serde_json::to_vec(&json!({
        "name": "audit_logs",
        "type": "base",
        "realtime": false,
        "schema": [
            { "name": "action", "type": "text", "required": true }
        ]
    }))
    .unwrap();

    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/collections")
                .header("Content-Type", "application/json")
                .body(Body::from(col_body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);

    // 2. Insert record into `audit_logs` (realtime is false -> NO SSE event should be emitted)
    let rec_body = serde_json::to_vec(&json!({ "action": "secret_event" })).unwrap();
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/collections/audit_logs/records")
                .header("Content-Type", "application/json")
                .body(Body::from(rec_body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);
    assert!(
        rx.try_recv().is_err(),
        "Expected no realtime broadcast when collection realtime is false"
    );

    // 3. Enable realtime on `audit_logs` via PATCH /api/collections/audit_logs
    let update_col_body = serde_json::to_vec(&json!({
        "name": "audit_logs",
        "type": "base",
        "realtime": true,
        "schema": [
            { "name": "action", "type": "text", "required": true }
        ]
    }))
    .unwrap();
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri("/api/collections/audit_logs")
                .header("Content-Type", "application/json")
                .body(Body::from(update_col_body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // 4. Insert record now that realtime is true -> SSE event MUST be emitted
    let rec_body2 = serde_json::to_vec(&json!({ "action": "live_event" })).unwrap();
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/collections/audit_logs/records")
                .header("Content-Type", "application/json")
                .body(Body::from(rec_body2))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);

    let msg = rx
        .try_recv()
        .expect("Expected realtime broadcast when collection realtime is true");
    assert_eq!(msg.collection, "audit_logs");
    assert_eq!(msg.action, "create");

    let _ = std::fs::remove_dir_all(&test_dir);
}

