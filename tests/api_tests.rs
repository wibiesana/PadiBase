use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use PadiBase::{api::router::create_router, AppState};
use serde_json::json;
use tower::ServiceExt;

#[tokio::test]
async fn test_full_padi_baas_flow() {
    let test_dir = std::path::PathBuf::from(format!("./target/test_data_{}", uuid::Uuid::new_v4().simple()));
    let _ = std::fs::remove_dir_all(&test_dir);
    let sqlite_url = format!("sqlite://{}/data.db", test_dir.display().to_string().replace('\\', "/"));
    let state = AppState::new_with_db_url(&test_dir, Some(&sqlite_url)).await.expect("init app state");
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

    let login_bytes = axum::body::to_bytes(res.into_body(), 1024 * 1024).await.unwrap();
    let login_val: serde_json::Value = serde_json::from_slice(&login_bytes).unwrap();
    let admin_token = login_val["item"]["token"].as_str().or_else(|| login_val["data"]["token"].as_str()).or_else(|| login_val["token"].as_str()).unwrap().to_string();

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
                .header("Authorization", format!("Bearer {}", admin_token))
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
    use PadiBase::core::db::{resolve_db_config, DbDriver};
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
    let sqlite_url = format!("sqlite://{}/data.db", test_dir.display().to_string().replace('\\', "/"));
    let state = AppState::new_with_db_url(&test_dir, Some(&sqlite_url)).await.expect("init app state");
    let mut rx = state.realtime.subscribe();
    let app = create_router(state);

    // 1. Create collection with `realtime: false`
    let col_body = serde_json::to_vec(&json!({
        "name": "audit_logs",
        "type": "base",
        "realtime": false,
        "createRule": "public",
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
        "createRule": "public",
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



#[tokio::test]
async fn test_all_records_and_padi_rust_envelope() {
    let test_dir = std::path::PathBuf::from(format!(
        "./target/test_all_{}",
        uuid::Uuid::new_v4().simple()
    ));
    let _ = std::fs::remove_dir_all(&test_dir);
    let sqlite_url = format!("sqlite://{}/data.db", test_dir.display().to_string().replace('\\', "/"));
    let state = AppState::new_with_db_url(&test_dir, Some(&sqlite_url)).await.expect("init app state");
    let app = create_router(state);

    // Create collection `posts`
    let col_body = serde_json::to_vec(&json!({
        "name": "posts",
        "type": "base",
        "createRule": "public",
        "schema": [
            { "name": "title", "type": "text", "required": true }
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

    // Create 2 records
    for i in 1..=2 {
        let rec_body = serde_json::to_vec(&json!({ "title": format!("Post {}", i) })).unwrap();
        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/collections/posts/records")
                    .header("Content-Type", "application/json")
                    .body(Body::from(rec_body))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::CREATED);
    }

    // 1. Test /api/collections/posts/records/all
    let res_all1 = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/collections/posts/records/all")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res_all1.status(), StatusCode::OK);
    let bytes1 = axum::body::to_bytes(res_all1.into_body(), 1024 * 1024).await.unwrap();
    let json_all1: serde_json::Value = serde_json::from_slice(&bytes1).unwrap();
    assert_eq!(json_all1["success"], true);
    assert_eq!(json_all1["item"].as_array().unwrap().len(), 2);

    // 2. Test /api/collections/posts/all shortcut
    let res_all2 = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/collections/posts/all")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res_all2.status(), StatusCode::OK);
    let bytes2 = axum::body::to_bytes(res_all2.into_body(), 1024 * 1024).await.unwrap();
    let json_all2: serde_json::Value = serde_json::from_slice(&bytes2).unwrap();
    assert_eq!(json_all2["success"], true);
    assert_eq!(json_all2["item"].as_array().unwrap().len(), 2);

    // 3. Test /api/posts/all direct shortcut with search
    let res_all3 = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/posts/all?search=Post%201")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res_all3.status(), StatusCode::OK);
    let bytes3 = axum::body::to_bytes(res_all3.into_body(), 1024 * 1024).await.unwrap();
    let json_all3: serde_json::Value = serde_json::from_slice(&bytes3).unwrap();
    assert_eq!(json_all3["success"], true);
    assert_eq!(json_all3["item"].as_array().unwrap().len(), 1);
    assert_eq!(json_all3["item"][0]["title"], "Post 1");

    let _ = std::fs::remove_dir_all(&test_dir);
}

#[tokio::test]
async fn test_eager_load_relations_student() {
    let test_dir = std::path::PathBuf::from(format!(
        "./target/test_rel_{}",
        uuid::Uuid::new_v4().simple()
    ));
    let _ = std::fs::remove_dir_all(&test_dir);
    let sqlite_url = format!("sqlite://{}/data.db", test_dir.display().to_string().replace('\\', "/"));
    let state = AppState::new_with_db_url(&test_dir, Some(&sqlite_url))
        .await
        .expect("init app state");

    // 1. Create table `students` (with id and name)
    state
        .db
        .execute(
            r#"
            CREATE TABLE students (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name VARCHAR(100) NOT NULL
            );
            "#,
            &[],
        )
        .await
        .expect("create students table");

    // 2. Create table `grades` (with id, student_id, score)
    state
        .db
        .execute(
            r#"
            CREATE TABLE grades (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                student_id INTEGER NOT NULL,
                score INTEGER NOT NULL,
                FOREIGN KEY (student_id) REFERENCES students(id)
            );
            "#,
            &[],
        )
        .await
        .expect("create grades table");

    // Insert student data
    state
        .db
        .execute("INSERT INTO students (name) VALUES ('John Doe');", &[])
        .await
        .expect("insert student");

    // Insert grade record referencing student_id = 1
    state
        .db
        .execute("INSERT INTO grades (student_id, score) VALUES (1, 95);", &[])
        .await
        .expect("insert grade");

    let app = create_router(state);

    // Sync collections
    let _ = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/collections/sync")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // 3. Request grades via /api/collections/grades/records/all (default auto_load_relations is false)
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/collections/grades/records/all")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);
    let body = axum::body::to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let val: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let items = val["item"].as_array().expect("expected items array");
    assert_eq!(items.len(), 1);
    let first = &items[0];
    assert_eq!(first["student_id"], 1);
    // Verify default: relation is NOT automatically loaded when table setting is false
    assert!(first.get("student").is_none() || first["student"].is_null());

    // 4. Request with explicit query parameter ?with=student
    let res_with = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/collections/grades/records/all?with=student")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res_with.status(), StatusCode::OK);
    let body_with = axum::body::to_bytes(res_with.into_body(), usize::MAX).await.unwrap();
    let val_with: serde_json::Value = serde_json::from_slice(&body_with).unwrap();
    let items_with = val_with["item"].as_array().expect("expected items array");
    let first_with = &items_with[0];
    assert!(first_with["student"].is_object(), "Student relation must be present when requested via ?with=student");
    assert_eq!(first_with["student"]["name"], "John Doe");

    // 5. Update collection setting on table: autoLoadRelations = true (simulate Dashboard toggle)
    let patch_payload = serde_json::to_vec(&json!({
        "name": "grades",
        "type": "base",
        "autoLoadRelations": true,
        "schema": []
    }))
    .unwrap();
    let res_patch = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri("/api/collections/grades")
                .header("Content-Type", "application/json")
                .body(Body::from(patch_payload))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res_patch.status(), StatusCode::OK);

    // 6. Request grades again WITHOUT query param: now should automatically eager load student relation
    let res_auto = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/collections/grades/records/all")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res_auto.status(), StatusCode::OK);
    let body_auto = axum::body::to_bytes(res_auto.into_body(), usize::MAX).await.unwrap();
    let val_auto: serde_json::Value = serde_json::from_slice(&body_auto).unwrap();
    let items_auto = val_auto["item"].as_array().expect("expected items array");
    let first_auto = &items_auto[0];
    assert!(first_auto["student"].is_object(), "Student relation must be loaded automatically when autoLoadRelations is true");
    assert_eq!(first_auto["student"]["name"], "John Doe");

    // 7. Reset per-table setting to false, but enable global .env (AUTO_LOAD_RELATIONS=true)
    let patch_payload2 = serde_json::to_vec(&json!({
        "name": "grades",
        "type": "base",
        "autoLoadRelations": false,
        "schema": []
    }))
    .unwrap();
    let _ = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri("/api/collections/grades")
                .header("Content-Type", "application/json")
                .body(Body::from(patch_payload2))
                .unwrap(),
        )
        .await
        .unwrap();

    PadiBase::core::env::Env::set("AUTO_LOAD_RELATIONS", "true");
    let res_global_env = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/collections/grades/records/all")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res_global_env.status(), StatusCode::OK);
    let body_global = axum::body::to_bytes(res_global_env.into_body(), usize::MAX).await.unwrap();
    let val_global: serde_json::Value = serde_json::from_slice(&body_global).unwrap();
    let items_global = val_global["item"].as_array().expect("expected items array");
    let first_global = &items_global[0];
    assert!(first_global["student"].is_object(), "Student relation must be loaded when global .env AUTO_LOAD_RELATIONS=true");
    assert_eq!(first_global["student"]["name"], "John Doe");

    // Reset env var to false
    PadiBase::core::env::Env::set("AUTO_LOAD_RELATIONS", "false");

    // ================= Test API Collection Export Endpoints =================
    // 1. Export Master API Collection
    let res_coll = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/api-collection")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res_coll.status(), StatusCode::OK);
    assert_eq!(
        res_coll.headers().get("content-type").unwrap(),
        "application/json; charset=utf-8"
    );
    let body_bytes = axum::body::to_bytes(res_coll.into_body(), usize::MAX).await.unwrap();
    let coll_json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    assert!(coll_json.get("info").is_some());
    assert_eq!(coll_json["info"]["name"], "PadiBase API Collection (Full)");
    assert_eq!(coll_json["info"]["schema"], "https://schema.getpostman.com/json/collection/v2.1.0/collection.json");
    assert!(coll_json.get("item").is_some());

    // 2. Export Master with ?download=true
    let res_dl = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/api-collection?download=true")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res_dl.status(), StatusCode::OK);
    assert!(res_dl
        .headers()
        .get("content-disposition")
        .unwrap()
        .to_str()
        .unwrap()
        .contains("attachment; filename=\"padi_base_api_collection.json\""));

    // 3. Export Single Collection (grades)
    let res_single = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/collections/grades/api-collection?download=true")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res_single.status(), StatusCode::OK);
    assert!(res_single
        .headers()
        .get("content-disposition")
        .unwrap()
        .to_str()
        .unwrap()
        .contains("attachment; filename=\"grades_collection.json\""));
    let body_single_bytes = axum::body::to_bytes(res_single.into_body(), usize::MAX).await.unwrap();
    let single_json: serde_json::Value = serde_json::from_slice(&body_single_bytes).unwrap();
    assert_eq!(single_json["info"]["name"], "Grades API Collection (PadiBase)");
    assert!(single_json["item"].as_array().unwrap().len() >= 6);

    let _ = std::fs::remove_dir_all(&test_dir);
}

#[tokio::test]
async fn test_file_upload_download_and_delete() {
    let test_dir = std::path::PathBuf::from(format!(
        "./target/test_file_{}",
        uuid::Uuid::new_v4().simple()
    ));
    let _ = std::fs::remove_dir_all(&test_dir);
    let sqlite_url = format!("sqlite://{}/data.db", test_dir.display().to_string().replace('\\', "/"));
    let state = AppState::new_with_db_url(&test_dir, Some(&sqlite_url))
        .await
        .expect("init app state");
    let app = create_router(state);

    // 1. Create collection `documents` with fields `title` and `file`
    let col_body = serde_json::to_vec(&json!({
        "name": "documents",
        "type": "base",
        "createRule": "public",
        "updateRule": "public",
        "deleteRule": "public",
        "schema": [
            { "name": "title", "type": "text", "required": true },
            { "name": "file", "type": "file", "required": false }
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

    // 2. Create a record first
    let rec_body = serde_json::to_vec(&json!({ "title": "Quarterly Report" })).unwrap();
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/collections/documents/records")
                .header("Content-Type", "application/json")
                .body(Body::from(rec_body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);
    let bytes = axum::body::to_bytes(res.into_body(), 1024 * 1024).await.unwrap();
    let rec_json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let record_id = rec_json["item"]["id"].as_str().or_else(|| rec_json["id"].as_str()).unwrap().to_string();

    // 3. Upload a file to POST /api/files/documents/{record_id}
    let boundary = "------------------------testboundary123";
    let file_content = b"PadiBase File Storage Content - 2026";
    let mut multipart_body = Vec::new();
    multipart_body.extend_from_slice(format!("--{}\r\n", boundary).as_bytes());
    multipart_body.extend_from_slice(b"Content-Disposition: form-data; name=\"file\"; filename=\"report.txt\"\r\n");
    multipart_body.extend_from_slice(b"Content-Type: text/plain\r\n\r\n");
    multipart_body.extend_from_slice(file_content);
    multipart_body.extend_from_slice(format!("\r\n--{}--\r\n", boundary).as_bytes());

    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/files/documents/{}", record_id))
                .header("Content-Type", format!("multipart/form-data; boundary={}", boundary))
                .body(Body::from(multipart_body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let upload_bytes = axum::body::to_bytes(res.into_body(), 1024 * 1024).await.unwrap();
    let upload_json: serde_json::Value = serde_json::from_slice(&upload_bytes).unwrap();
    let uploaded_item = &upload_json["item"];
    let saved_filename = uploaded_item["uploaded"][0]["filename"].as_str().unwrap().to_string();
    assert!(saved_filename.ends_with("_report.txt"));

    // 4. Download file via GET /api/files/documents/{record_id}/{saved_filename}
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/files/documents/{}/{}", record_id, saved_filename))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(
        res.headers().get("content-type").unwrap().to_str().unwrap(),
        "text/plain"
    );
    let downloaded_bytes = axum::body::to_bytes(res.into_body(), 1024 * 1024).await.unwrap();
    assert_eq!(downloaded_bytes.as_ref(), file_content);

    // 5. Delete file via DELETE /api/files/documents/{record_id}/{saved_filename}
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/files/documents/{}/{}", record_id, saved_filename))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // 6. Verify file is gone (404 Not Found)
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/files/documents/{}/{}", record_id, saved_filename))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);

    // 7. Test direct multipart create: POST /api/collections/documents/records with file attached
    let boundary2 = "------------------------directboundary456";
    let img_content = b"\x89PNG\r\n\x1a\nFakePngData";
    let mut direct_body = Vec::new();
    direct_body.extend_from_slice(format!("--{}\r\n", boundary2).as_bytes());
    direct_body.extend_from_slice(b"Content-Disposition: form-data; name=\"title\"\r\n\r\n");
    direct_body.extend_from_slice(b"Document with Direct File\r\n");
    direct_body.extend_from_slice(format!("--{}\r\n", boundary2).as_bytes());
    direct_body.extend_from_slice(b"Content-Disposition: form-data; name=\"file\"; filename=\"avatar.png\"\r\n");
    direct_body.extend_from_slice(b"Content-Type: image/png\r\n\r\n");
    direct_body.extend_from_slice(img_content);
    direct_body.extend_from_slice(format!("\r\n--{}--\r\n", boundary2).as_bytes());

    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/collections/documents/records")
                .header("Content-Type", format!("multipart/form-data; boundary={}", boundary2))
                .body(Body::from(direct_body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);
    let direct_bytes = axum::body::to_bytes(res.into_body(), 1024 * 1024).await.unwrap();
    let direct_json: serde_json::Value = serde_json::from_slice(&direct_bytes).unwrap();
    let direct_item = &direct_json["item"];
    assert_eq!(direct_item["title"], "Document with Direct File");
    let direct_file = direct_item["file"].as_str().unwrap();
    assert!(direct_file.ends_with("_avatar.png"));

    // Download the directly created file
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/files/documents/{}/{}", direct_item["id"].as_str().unwrap(), direct_file))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let downloaded_img = axum::body::to_bytes(res.into_body(), 1024 * 1024).await.unwrap();
    assert_eq!(downloaded_img.as_ref(), img_content);

    let _ = std::fs::remove_dir_all(&test_dir);
}

#[tokio::test]
async fn test_settings_reload_in_memory() {
    let test_dir = std::path::PathBuf::from(format!(
        "./target/test_settings_{}",
        uuid::Uuid::new_v4().simple()
    ));
    let _ = std::fs::remove_dir_all(&test_dir);
    let sqlite_url = format!("sqlite://{}/data.db", test_dir.display().to_string().replace('\\', "/"));
    let state = AppState::new_with_db_url(&test_dir, Some(&sqlite_url))
        .await
        .expect("init app state");
    let app = create_router(state);

    // 1. Initial settings
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/settings")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(res.into_body(), 1024 * 1024).await.unwrap();
    let json_val: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert!(json_val["item"]["meta"]["appName"].is_string());

    // 2. User updates setting
    let updated_payload = json!({
        "meta": {
            "appName": "Super Padi App",
            "appUrl": "https://myapp.com",
            "hideControls": true
        }
    });
    let patch_body = serde_json::to_vec(&updated_payload).unwrap();

    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri("/api/settings")
                .header("Content-Type", "application/json")
                .body(Body::from(patch_body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // 3. Get settings again: must reflect updated values reloaded into memory
    let res2 = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/settings")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res2.status(), StatusCode::OK);
    let bytes2 = axum::body::to_bytes(res2.into_body(), 1024 * 1024).await.unwrap();
    let json_val2: serde_json::Value = serde_json::from_slice(&bytes2).unwrap();
    assert_eq!(json_val2["item"]["meta"]["appName"], "Super Padi App");
    assert_eq!(json_val2["item"]["meta"]["hideControls"], true);

    let _ = std::fs::remove_dir_all(&test_dir);
}


#[tokio::test]
async fn test_auth_flow_and_transactions() {
    let test_dir = std::path::PathBuf::from(format!(
        "./target/test_tx_{}",
        uuid::Uuid::new_v4().simple()
    ));
    let _ = std::fs::remove_dir_all(&test_dir);
    let sqlite_url = format!(
        "sqlite://{}/data.db",
        test_dir.display().to_string().replace('\\', "/")
    );
    let state = AppState::new_with_db_url(&test_dir, Some(&sqlite_url))
        .await
        .expect("init app state");
    let app = create_router(state);

    // 1. Setup Admin
    let admin_body = serde_json::to_vec(&json!({
        "email": "admin@tx.local",
        "password": "adminpassword123"
    }))
    .unwrap();
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/admins")
                .header("Content-Type", "application/json")
                .body(Body::from(admin_body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);

    let login_admin = serde_json::to_vec(&json!({
        "identity": "admin@tx.local",
        "password": "adminpassword123"
    }))
    .unwrap();
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/admins/auth-with-password")
                .header("Content-Type", "application/json")
                .body(Body::from(login_admin))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(res.into_body(), 1024 * 1024).await.unwrap();
    let val: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let admin_token = val["item"]["token"].as_str().unwrap().to_string();

    // 2. Test Register Endpoint (`POST /api/auth/register`)
    let reg_body = serde_json::to_vec(&json!({
        "email": "budi@example.com",
        "username": "budisantoso",
        "name": "Budi Santoso",
        "password": "PasswordAwal123",
        "passwordConfirm": "PasswordAwal123"
    }))
    .unwrap();
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/register")
                .header("Content-Type", "application/json")
                .body(Body::from(reg_body.clone()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);
    let reg_bytes = axum::body::to_bytes(res.into_body(), 1024 * 1024).await.unwrap();
    let reg_val: serde_json::Value = serde_json::from_slice(&reg_bytes).unwrap();
    let user_token = reg_val["item"]["token"].as_str().unwrap().to_string();
    assert_eq!(reg_val["item"]["record"]["email"], "budi@example.com");

    // Duplicate email registration must return 400
    let res_dup = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/register")
                .header("Content-Type", "application/json")
                .body(Body::from(reg_body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res_dup.status(), StatusCode::BAD_REQUEST);

    // 3. Test `/api/auth/me`
    let res_me = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/auth/me")
                .header("Authorization", format!("Bearer {}", user_token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res_me.status(), StatusCode::OK);

    // 4. Test Forgot Password & Reset Password
    let forgot_body = serde_json::to_vec(&json!({
        "email": "budi@example.com"
    }))
    .unwrap();
    let res_forgot = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/forgot-password")
                .header("Content-Type", "application/json")
                .body(Body::from(forgot_body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res_forgot.status(), StatusCode::OK);
    let forgot_bytes = axum::body::to_bytes(res_forgot.into_body(), 1024 * 1024).await.unwrap();
    let forgot_val: serde_json::Value = serde_json::from_slice(&forgot_bytes).unwrap();
    let reset_token = forgot_val["item"]["reset_token"].as_str().unwrap().to_string();

    let reset_body = serde_json::to_vec(&json!({
        "token": reset_token,
        "password": "PasswordBaru456",
        "passwordConfirm": "PasswordBaru456"
    }))
    .unwrap();
    let res_reset = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/reset-password")
                .header("Content-Type", "application/json")
                .body(Body::from(reset_body.clone()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res_reset.status(), StatusCode::OK);

    // Reusing the same reset token must fail (single-use rotation)
    let res_reset_reuse = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/reset-password")
                .header("Content-Type", "application/json")
                .body(Body::from(reset_body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res_reset_reuse.status(), StatusCode::BAD_REQUEST);

    // Login with new password must succeed
    let login_new = serde_json::to_vec(&json!({
        "email": "budi@example.com",
        "password": "PasswordBaru456"
    }))
    .unwrap();
    let res_login_new = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/login")
                .header("Content-Type", "application/json")
                .body(Body::from(login_new))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res_login_new.status(), StatusCode::OK);

    // 5. Test Change Password while logged in
    let change_pw_body = serde_json::to_vec(&json!({
        "oldPassword": "PasswordBaru456",
        "newPassword": "PasswordFinal789",
        "passwordConfirm": "PasswordFinal789"
    }))
    .unwrap();
    let res_change = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/change-password")
                .header("Content-Type", "application/json")
                .header("Authorization", format!("Bearer {}", user_token))
                .body(Body::from(change_pw_body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res_change.status(), StatusCode::OK);

    // 6. Test Atomic Transactions (`POST /api/transactions`) & Custom Transfer (`POST /api/custom/transfer`)
    let col_wallets = serde_json::to_vec(&json!({
        "name": "wallets",
        "type": "base",
        "schema": [
            { "name": "owner", "type": "text", "required": true },
            { "name": "balance", "type": "number", "required": true }
        ]
    }))
    .unwrap();
    let res_col = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/collections")
                .header("Content-Type", "application/json")
                .header("Authorization", format!("Bearer {}", admin_token))
                .body(Body::from(col_wallets))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res_col.status(), StatusCode::CREATED);

    // 6a. Execute batch transaction creating 2 wallets atomically
    let tx_create = serde_json::to_vec(&json!({
        "operations": [
            {
                "action": "create",
                "collection": "wallets",
                "data": { "id": "wallet_a", "owner": "Alice", "balance": 1000 }
            },
            {
                "action": "create",
                "collection": "wallets",
                "data": { "id": "wallet_b", "owner": "Bob", "balance": 500 }
            }
        ]
    }))
    .unwrap();
    let res_tx = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/transactions")
                .header("Content-Type", "application/json")
                .header("Authorization", format!("Bearer {}", user_token))
                .body(Body::from(tx_create))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res_tx.status(), StatusCode::OK);

    // 6b. Test atomic rollback when step 2 fails (updating wallet_a then updating non-existent wallet_z)
    let tx_fail = serde_json::to_vec(&json!({
        "operations": [
            {
                "action": "update",
                "collection": "wallets",
                "id": "wallet_a",
                "data": { "balance": 999999 }
            },
            {
                "action": "update",
                "collection": "wallets",
                "id": "non_existent_wallet",
                "data": { "balance": 100 }
            }
        ]
    }))
    .unwrap();
    let res_tx_fail = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/transactions")
                .header("Content-Type", "application/json")
                .header("Authorization", format!("Bearer {}", user_token))
                .body(Body::from(tx_fail))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res_tx_fail.status(), StatusCode::NOT_FOUND);

    // Verify wallet_a balance is STILL 1000 (rolled back!)
    let res_check = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/collections/wallets/records/wallet_a")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let check_bytes = axum::body::to_bytes(res_check.into_body(), 1024 * 1024).await.unwrap();
    let check_val: serde_json::Value = serde_json::from_slice(&check_bytes).unwrap();
    assert_eq!(check_val["item"]["balance"].as_f64(), Some(1000.0));

    // 7. Test Custom Atomic Transfer (`POST /api/custom/transfer`: 250 from wallet_a to wallet_b)
    let transfer_body = serde_json::to_vec(&json!({
        "collection": "wallets",
        "from_id": "wallet_a",
        "to_id": "wallet_b",
        "field": "balance",
        "amount": 250
    }))
    .unwrap();
    let res_transfer = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/custom/transfer")
                .header("Content-Type", "application/json")
                .header("Authorization", format!("Bearer {}", user_token))
                .body(Body::from(transfer_body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res_transfer.status(), StatusCode::OK);

    // 8. Test Admin Custom SQL Query (`POST /api/sql`) to verify total balance is still 1500
    let sql_body = serde_json::to_vec(&json!({
        "sql": "SELECT SUM(`balance`) as `total_balance` FROM `wallets` WHERE `balance` > ?",
        "params": [0]
    }))
    .unwrap();
    let res_sql = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/sql")
                .header("Content-Type", "application/json")
                .header("Authorization", format!("Bearer {}", admin_token))
                .body(Body::from(sql_body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res_sql.status(), StatusCode::OK);
    let sql_bytes = axum::body::to_bytes(res_sql.into_body(), 1024 * 1024).await.unwrap();
    let sql_val: serde_json::Value = serde_json::from_slice(&sql_bytes).unwrap();
    let sql_rows = if sql_val.get("item").and_then(|v| v.as_array()).is_some() {
        &sql_val["item"]
    } else {
        &sql_val["items"]
    };
    let total_num = sql_rows[0]["total_balance"]
        .as_f64()
        .or_else(|| sql_rows[0]["total_balance"].as_i64().map(|v| v as f64));
    assert_eq!(total_num, Some(1500.0));

    // 9. Test Dynamic Custom SQL Endpoints saved in DB (`POST /api/endpoints` & `GET /api/custom/wallet-summary`)
    let create_ep_body = serde_json::to_vec(&json!({
        "name": "Wallet Summary",
        "path": "wallet-summary",
        "method": "GET",
        "sql": "SELECT `owner`, `balance` FROM `wallets` WHERE `balance` >= :min_balance ORDER BY `balance` DESC;",
        "description": "Get wallets above minimum balance",
        "authRule": "public",
        "transaction": false,
        "singleRow": false
    }))
    .unwrap();
    let res_create_ep = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/endpoints")
                .header("Content-Type", "application/json")
                .header("Authorization", format!("Bearer {}", admin_token))
                .body(Body::from(create_ep_body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res_create_ep.status(), StatusCode::CREATED);

    // Call the newly created dynamic endpoint: `GET /api/custom/wallet-summary?min_balance=700`
    let res_exec_ep = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/custom/wallet-summary?min_balance=700")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res_exec_ep.status(), StatusCode::OK);
    let ep_bytes = axum::body::to_bytes(res_exec_ep.into_body(), 1024 * 1024).await.unwrap();
    let ep_val: serde_json::Value = serde_json::from_slice(&ep_bytes).unwrap();
    let items = ep_val.get("item").or(ep_val.get("items")).and_then(|v| v.as_array()).unwrap();
    assert_eq!(items.len(), 2);

    // 10. Test Dynamic Custom SQL Endpoint with LIMIT 1 automatically returning single row object
    let create_ep_limit1_body = serde_json::to_vec(&json!({
        "name": "Top Wallet",
        "path": "top-wallet",
        "method": "GET",
        "sql": "SELECT `owner`, `balance` FROM `wallets` ORDER BY `balance` DESC LIMIT 1;",
        "description": "Get top wallet"
    }))
    .unwrap();
    let res_create_limit1 = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/endpoints")
                .header("Content-Type", "application/json")
                .header("Authorization", format!("Bearer {}", admin_token))
                .body(Body::from(create_ep_limit1_body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res_create_limit1.status(), StatusCode::CREATED);

    let res_exec_limit1 = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/custom/top-wallet")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res_exec_limit1.status(), StatusCode::OK);
    let limit1_bytes = axum::body::to_bytes(res_exec_limit1.into_body(), 1024 * 1024).await.unwrap();
    let limit1_val: serde_json::Value = serde_json::from_slice(&limit1_bytes).unwrap();
    // Since it's single row via LIMIT 1 in SQL, it returns the object directly { "owner": ..., "balance": ... }
    assert!(limit1_val.is_object());
    assert_eq!(limit1_val["owner"], "Charlie");

    let _ = std::fs::remove_dir_all(&test_dir);
}
