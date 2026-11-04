use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use PadiBase::{api::router::create_router, AppState};
use serde_json::json;
use tower::ServiceExt;

#[tokio::test]
async fn test_default_rules_require_authenticated_user() {
    let test_dir = std::path::PathBuf::from(format!(
        "./target/test_auth_rules_{}",
        uuid::Uuid::new_v4().simple()
    ));
    let _ = std::fs::remove_dir_all(&test_dir);
    let sqlite_url = format!("sqlite://{}/data.db", test_dir.display().to_string().replace('\\', "/"));
    let state = AppState::new_with_db_url(&test_dir, Some(&sqlite_url)).await.expect("init app state");
    let app = create_router(state);

    // 1. Create admin user
    let req_body = serde_json::to_vec(&json!({
        "email": "admin@example.com",
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

    // 2. Login admin to obtain JWT
    let login_body = serde_json::to_vec(&json!({
        "identity": "admin@example.com",
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
    let login_json: serde_json::Value = serde_json::from_slice(&login_bytes).unwrap();
    let admin_token = login_json["item"]["token"].as_str().or_else(|| login_json["token"].as_str()).unwrap().to_string();

    // 3. Create collection `articles` with DEFAULT rules (no explicit createRule/updateRule/deleteRule specified)
    let col_body = serde_json::to_vec(&json!({
        "name": "articles",
        "type": "base",
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
                .header("Authorization", format!("Bearer {}", admin_token))
                .body(Body::from(col_body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);

    // 4. Try to CREATE record as ANONYMOUS (no token) -> MUST BE REJECTED WITH 401 UNAUTHORIZED!
    let rec_body = serde_json::to_vec(&json!({ "title": "Secret article" })).unwrap();
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/collections/articles/records")
                .header("Content-Type", "application/json")
                .body(Body::from(rec_body.clone()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);

    // 5. CREATE record with valid Bearer token -> MUST SUCCEED (201 CREATED)
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/collections/articles/records")
                .header("Content-Type", "application/json")
                .header("Authorization", format!("Bearer {}", admin_token))
                .body(Body::from(rec_body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);
    let created_bytes = axum::body::to_bytes(res.into_body(), 1024 * 1024).await.unwrap();
    let created_json: serde_json::Value = serde_json::from_slice(&created_bytes).unwrap();
    let article_id = created_json["id"].as_str().or_else(|| created_json["item"]["id"].as_str()).unwrap().to_string();

    // 6. Try to UPDATE record as ANONYMOUS -> MUST BE REJECTED WITH 401 UNAUTHORIZED!
    let update_body = serde_json::to_vec(&json!({ "title": "Updated article anonymously" })).unwrap();
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri(format!("/api/collections/articles/records/{}", article_id))
                .header("Content-Type", "application/json")
                .body(Body::from(update_body.clone()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);

    // 7. UPDATE record with valid Bearer token -> MUST SUCCEED (200 OK)
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri(format!("/api/collections/articles/records/{}", article_id))
                .header("Content-Type", "application/json")
                .header("Authorization", format!("Bearer {}", admin_token))
                .body(Body::from(update_body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // 8. Try to DELETE record as ANONYMOUS -> MUST BE REJECTED WITH 401 UNAUTHORIZED!
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/collections/articles/records/{}", article_id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);

    // 9. Now test "user settings otherwise": create collection `public_notes` with `createRule: "public"`
    let pub_col_body = serde_json::to_vec(&json!({
        "name": "public_notes",
        "type": "base",
        "createRule": "public",
        "updateRule": "public",
        "deleteRule": "public",
        "schema": [
            { "name": "note", "type": "text", "required": true }
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
                .header("Authorization", format!("Bearer {}", admin_token))
                .body(Body::from(pub_col_body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);

    // 10. ANONYMOUS user CAN now create record because user configured it to "public"
    let note_body = serde_json::to_vec(&json!({ "note": "Hello World from Anonymous!" })).unwrap();
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/collections/public_notes/records")
                .header("Content-Type", "application/json")
                .body(Body::from(note_body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);

    // Clean up test data
    let _ = std::fs::remove_dir_all(&test_dir);
}

#[tokio::test]
async fn test_role_based_access_rules() {
    let test_dir = std::path::PathBuf::from(format!(
        "./target/test_roles_{}",
        uuid::Uuid::new_v4().simple()
    ));
    let _ = std::fs::remove_dir_all(&test_dir);
    let sqlite_url = format!("sqlite://{}/data.db", test_dir.display().to_string().replace('\\', "/"));
    let state = AppState::new_with_db_url(&test_dir, Some(&sqlite_url)).await.expect("init app state");
    let admin_token = PadiBase::core::auth::create_jwt("admin_roles", "admin@roles.local", "admin", None, 7).unwrap();
    let app = create_router(state);

    // 1. Create collection `chapters` with `createRule: "@request.auth.role in ['admin', 'creator']"`
    let col_body = serde_json::to_vec(&json!({
        "name": "chapters",
        "type": "base",
        "createRule": "@request.auth.role in ['admin', 'creator']",
        "updateRule": "@request.auth.role = 'creator'",
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
                .header("Authorization", format!("Bearer {}", admin_token))
                .body(Body::from(col_body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);

    // 2. Register regular user (role: 'user') in users collection
    let user_pwd_hash = PadiBase::core::auth::hash_password("password123").unwrap();
    let user_rec = serde_json::to_vec(&json!({
        "email": "reader@example.com",
        "username": "reader",
        "role": "user",
        "passwordHash": user_pwd_hash,
    }))
    .unwrap();
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/collections/users/records")
                .header("Content-Type", "application/json")
                .header("Authorization", format!("Bearer {}", admin_token))
                .body(Body::from(user_rec))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);

    // 3. Register creator user (role: 'creator') in users collection
    let creator_pwd_hash = PadiBase::core::auth::hash_password("password123").unwrap();
    let creator_rec = serde_json::to_vec(&json!({
        "email": "writer@example.com",
        "username": "writer",
        "role": "creator",
        "passwordHash": creator_pwd_hash,
    }))
    .unwrap();
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/collections/users/records")
                .header("Content-Type", "application/json")
                .header("Authorization", format!("Bearer {}", admin_token))
                .body(Body::from(creator_rec))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);

    // 4. Login regular user to get JWT token
    let login_body = serde_json::to_vec(&json!({
        "identity": "reader@example.com",
        "password": "password123"
    }))
    .unwrap();
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/collections/users/auth-with-password")
                .header("Content-Type", "application/json")
                .body(Body::from(login_body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(res.into_body(), 1024 * 1024).await.unwrap();
    let login_json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let user_token = login_json["item"]["token"].as_str().or_else(|| login_json["token"].as_str()).unwrap().to_string();

    // 5. Login creator user to get JWT token
    let login_body2 = serde_json::to_vec(&json!({
        "identity": "writer@example.com",
        "password": "password123"
    }))
    .unwrap();
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/collections/users/auth-with-password")
                .header("Content-Type", "application/json")
                .body(Body::from(login_body2))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let bytes2 = axum::body::to_bytes(res.into_body(), 1024 * 1024).await.unwrap();
    let login_json2: serde_json::Value = serde_json::from_slice(&bytes2).unwrap();
    let creator_token = login_json2["item"]["token"].as_str().or_else(|| login_json2["token"].as_str()).unwrap().to_string();

    // 6. Test: Regular user (role 'user') tries to create chapter -> MUST BE REJECTED WITH 403 FORBIDDEN!
    let chapter_body = serde_json::to_vec(&json!({ "title": "Unauthorized Chapter" })).unwrap();
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/collections/chapters/records")
                .header("Content-Type", "application/json")
                .header("Authorization", format!("Bearer {}", user_token))
                .body(Body::from(chapter_body.clone()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);

    // 7. Test: Creator user (role 'creator') creates chapter -> MUST SUCCEED (201 CREATED)!
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/collections/chapters/records")
                .header("Content-Type", "application/json")
                .header("Authorization", format!("Bearer {}", creator_token))
                .body(Body::from(chapter_body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);

    let _ = std::fs::remove_dir_all(&test_dir);
}

#[tokio::test]
async fn test_owner_and_admin_delete_permission() {
    let test_dir = std::path::PathBuf::from(format!(
        "./target/test_owner_admin_{}",
        uuid::Uuid::new_v4().simple()
    ));
    let _ = std::fs::remove_dir_all(&test_dir);
    let sqlite_url = format!("sqlite://{}/data.db", test_dir.display().to_string().replace('\\', "/"));
    let state = AppState::new_with_db_url(&test_dir, Some(&sqlite_url)).await.expect("init app state");
    let app = create_router(state);

    // 1. Create superadmin to create collections
    let req_body = serde_json::to_vec(&json!({
        "email": "superadmin@example.com",
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

    let login_body = serde_json::to_vec(&json!({
        "identity": "superadmin@example.com",
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
    let bytes = axum::body::to_bytes(res.into_body(), 1024 * 1024).await.unwrap();
    let login_json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let admin_token = login_json["item"]["token"].as_str().or_else(|| login_json["token"].as_str()).unwrap().to_string();

    // 2. Create regular user A (role: 'user')
    let user_a_body = serde_json::to_vec(&json!({
        "email": "userA@example.com",
        "password": "password123",
        "passwordConfirm": "password123",
        "role": "user"
    }))
    .unwrap();
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/collections/users/records")
                .header("Content-Type", "application/json")
                .header("Authorization", format!("Bearer {}", admin_token))
                .body(Body::from(user_a_body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);
    let bytes = axum::body::to_bytes(res.into_body(), 1024 * 1024).await.unwrap();
    let user_a_rec: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let user_a_id = user_a_rec["id"].as_str().or_else(|| user_a_rec["item"]["id"].as_str()).unwrap().to_string();

    // 4. Create user B (role: 'user', another user)
    let user_b_body = serde_json::to_vec(&json!({
        "email": "userB@example.com",
        "password": "password123",
        "passwordConfirm": "password123",
        "role": "user"
    }))
    .unwrap();
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/collections/users/records")
                .header("Content-Type", "application/json")
                .header("Authorization", format!("Bearer {}", admin_token))
                .body(Body::from(user_b_body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);

    // 5. Create user C (role: 'admin' - regular auth account with role admin)
    let user_admin_body = serde_json::to_vec(&json!({
        "email": "appadmin@example.com",
        "password": "password123",
        "passwordConfirm": "password123",
        "role": "admin"
    }))
    .unwrap();
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/collections/users/records")
                .header("Content-Type", "application/json")
                .header("Authorization", format!("Bearer {}", admin_token))
                .body(Body::from(user_admin_body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);

    // Login user A
    let login_a = serde_json::to_vec(&json!({ "identity": "userA@example.com", "password": "password123" })).unwrap();
    let res = app.clone().oneshot(Request::builder().method("POST").uri("/api/collections/users/auth-with-password").header("Content-Type", "application/json").body(Body::from(login_a)).unwrap()).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(res.into_body(), 1024 * 1024).await.unwrap();
    let j: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let token_a = j["item"]["token"].as_str().or_else(|| j["token"].as_str()).unwrap().to_string();

    // Login user B
    let login_b = serde_json::to_vec(&json!({ "identity": "userB@example.com", "password": "password123" })).unwrap();
    let res = app.clone().oneshot(Request::builder().method("POST").uri("/api/collections/users/auth-with-password").header("Content-Type", "application/json").body(Body::from(login_b)).unwrap()).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(res.into_body(), 1024 * 1024).await.unwrap();
    let j: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let token_b = j["item"]["token"].as_str().or_else(|| j["token"].as_str()).unwrap().to_string();

    // Login app admin C
    let login_c = serde_json::to_vec(&json!({ "identity": "appadmin@example.com", "password": "password123" })).unwrap();
    let res = app.clone().oneshot(Request::builder().method("POST").uri("/api/collections/users/auth-with-password").header("Content-Type", "application/json").body(Body::from(login_c)).unwrap()).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(res.into_body(), 1024 * 1024).await.unwrap();
    let j: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let token_admin = j["item"]["token"].as_str().or_else(|| j["token"].as_str()).unwrap().to_string();

    // 6. Create collection `posts` with Delete Rule: `user_id = @request.auth.id || @request.auth.role = 'admin'`
    let posts_col_body = serde_json::to_vec(&json!({
        "name": "posts",
        "type": "base",
        "schema": [
            { "name": "title", "type": "text", "required": true },
            { "name": "user_id", "type": "text", "required": true }
        ],
        "listRule": "public",
        "viewRule": "public",
        "createRule": "@request.auth.id != ''",
        "updateRule": "user_id = @request.auth.id || @request.auth.role = 'admin'",
        "deleteRule": "user_id = @request.auth.id || @request.auth.role = 'admin'"
    }))
    .unwrap();
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/collections")
                .header("Content-Type", "application/json")
                .header("Authorization", format!("Bearer {}", admin_token))
                .body(Body::from(posts_col_body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);

    // 7. User A creates a post where user_id = user_a_id
    let post_body = serde_json::to_vec(&json!({
        "title": "Post by User A",
        "user_id": user_a_id
    }))
    .unwrap();
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/collections/posts/records")
                .header("Content-Type", "application/json")
                .header("Authorization", format!("Bearer {}", token_a))
                .body(Body::from(post_body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);
    let bytes = axum::body::to_bytes(res.into_body(), 1024 * 1024).await.unwrap();
    let post_rec: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let post_id = post_rec["id"].as_str().or_else(|| post_rec["item"]["id"].as_str()).unwrap().to_string();

    // 8. User B (not owner, not admin) tries to DELETE post -> MUST BE 403 FORBIDDEN!
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/collections/posts/records/{}", post_id))
                .header("Authorization", format!("Bearer {}", token_b))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);

    // 9. App Admin C (role: 'admin', not owner) tries to DELETE post -> MUST BE 204 NO_CONTENT (SUCCEED)!
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/collections/posts/records/{}", post_id))
                .header("Authorization", format!("Bearer {}", token_admin))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NO_CONTENT);

    // 10. User A creates another post
    let post_body2 = serde_json::to_vec(&json!({
        "title": "Second Post by User A",
        "user_id": user_a_id
    }))
    .unwrap();
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/collections/posts/records")
                .header("Content-Type", "application/json")
                .header("Authorization", format!("Bearer {}", token_a))
                .body(Body::from(post_body2))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);
    let bytes = axum::body::to_bytes(res.into_body(), 1024 * 1024).await.unwrap();
    let post_rec2: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let post_id2 = post_rec2["id"].as_str().or_else(|| post_rec2["item"]["id"].as_str()).unwrap().to_string();

    // 11. User A (owner, role is 'user', NOT admin) DELETE own post -> MUST BE 204 NO_CONTENT (SUCCEED)!
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/collections/posts/records/{}", post_id2))
                .header("Authorization", format!("Bearer {}", token_a))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NO_CONTENT);

    let _ = std::fs::remove_dir_all(&test_dir);
}
