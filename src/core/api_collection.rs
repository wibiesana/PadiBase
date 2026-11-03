use serde_json::{json, Value};
use uuid::Uuid;

use super::schema::{Collection, CollectionType, FieldType, SchemaField};

/// Convert string into PascalCase / TitleCase
pub fn pascal_case(s: &str) -> String {
    s.split(|c: char| c == '_' || c == '-' || c == ' ')
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
            }
        })
        .collect()
}

/// Derive singular name heuristically from a plural collection name
pub fn singular_name(s: &str) -> String {
    let lower = s.to_lowercase();
    if lower.ends_with("ies") && lower.len() > 3 {
        format!("{}y", &lower[..lower.len() - 3])
    } else if lower.ends_with("es") && lower.len() > 2 && (lower.ends_with("shes") || lower.ends_with("ches") || lower.ends_with("xes") || lower.ends_with("sses")) {
        lower[..lower.len() - 2].to_string()
    } else if lower.ends_with('s') && !lower.ends_with("ss") && lower.len() > 1 {
        lower[..lower.len() - 1].to_string()
    } else {
        lower
    }
}

/// Generate realistic sample dummy values for schema fields
pub fn sample_value_for_field(field: &SchemaField) -> Value {
    let name_lower = field.name.to_lowercase();

    match field.field_type {
        FieldType::Number => {
            if name_lower.contains("price")
                || name_lower.contains("amount")
                || name_lower.contains("cost")
                || name_lower.contains("salary")
                || name_lower.contains("fee")
                || name_lower.contains("rate")
            {
                json!(99.99)
            } else if name_lower.contains("qty")
                || name_lower.contains("quantity")
                || name_lower.contains("stock")
                || name_lower.contains("count")
                || name_lower.contains("age")
            {
                json!(10)
            } else if name_lower.contains("year") {
                json!(2026)
            } else {
                json!(1)
            }
        }
        FieldType::Bool => json!(true),
        FieldType::Email => json!("sample@example.com"),
        FieldType::Url => json!("https://example.com/sample.png"),
        FieldType::Date => json!("2026-01-01"),
        FieldType::Select => {
            if let Some(values) = field.options.get("values").and_then(|v| v.as_array()) {
                if let Some(first) = values.first().and_then(|v| v.as_str()) {
                    return json!(first);
                }
            }
            json!("option_1")
        }
        FieldType::Json => json!({ "key": "sample_value" }),
        FieldType::File => json!("sample_attachment.jpg"),
        FieldType::Relation => json!("rel_sample_id"),
        FieldType::Text => {
            if name_lower.contains("email") {
                json!("sample@example.com")
            } else if name_lower.contains("password") {
                json!("Secret123!@#")
            } else if name_lower.contains("phone") || name_lower.contains("telp") {
                json!("08123456789")
            } else if name_lower.contains("address") || name_lower.contains("alamat") {
                json!("Jl. Sudirman No. 123, Jakarta")
            } else if name_lower.contains("desc") || name_lower.contains("note") {
                json!(format!("Sample description for {}", field.name))
            } else if name_lower.ends_with("_url") || name_lower.contains("avatar") || name_lower.contains("photo") {
                json!("https://example.com/avatar.jpg")
            } else {
                json!(format!("Sample {}", pascal_case(&field.name)))
            }
        }
    }
}

/// Generate sample create and update JSON objects for a collection
pub fn generate_sample_payloads(col: &Collection) -> (Value, Value) {
    let mut create_map = serde_json::Map::new();
    let mut update_map = serde_json::Map::new();

    if col.collection_type == CollectionType::Auth {
        create_map.insert("username".to_string(), json!("user123"));
        create_map.insert("email".to_string(), json!("user@example.com"));
        create_map.insert("password".to_string(), json!("Password123!@#"));
        create_map.insert("passwordConfirm".to_string(), json!("Password123!@#"));

        update_map.insert("username".to_string(), json!("user123_updated"));
    }

    for f in &col.schema {
        let val = sample_value_for_field(f);
        create_map.insert(f.name.clone(), val.clone());
        if update_map.len() < 3 {
            update_map.insert(f.name.clone(), val);
        }
    }

    // Default fallback if collection schema is empty
    if create_map.is_empty() {
        create_map.insert("title".to_string(), json!(format!("Sample {}", pascal_case(&col.name))));
        create_map.insert("description".to_string(), json!("PadiBase sample record"));
    }
    if update_map.is_empty() {
        update_map.insert("title".to_string(), json!(format!("Updated {}", pascal_case(&col.name))));
    }

    (Value::Object(create_map), Value::Object(update_map))
}

/// Build list of REST request items for a single collection
pub fn build_collection_items(col: &Collection) -> Vec<Value> {
    let c_name = &col.name;
    let singular = singular_name(c_name);
    let title_name = pascal_case(c_name);
    let (sample_create, sample_update) = generate_sample_payloads(col);

    let create_body_raw = serde_json::to_string_pretty(&sample_create).unwrap_or_default();
    let update_body_raw = serde_json::to_string_pretty(&sample_update).unwrap_or_default();

    let id_var_key = format!("{}_id", singular);

    let record_id_test_script = format!(
        "try {{\n    if (pm.response.code === 201 || pm.response.code === 200) {{\n        const res = pm.response.json();\n        const item = res.data?.item || res.item || res.data || res;\n        const id = item?.id || res.id;\n        if (id) {{\n            pm.collectionVariables.set('{id_var_key}', id);\n            pm.collectionVariables.set('record_id', id);\n        }}\n    }}\n}} catch (e) {{\n    console.warn('Script bypassed:', e);\n}}"
    );

    let mut items = Vec::new();

    // 1. Auth Endpoints (if collection type is Auth)
    if col.collection_type == CollectionType::Auth {
        let auth_test_script = format!(
            "try {{\n    if (pm.response.code === 200 || pm.response.code === 201) {{\n        const res = pm.response.json();\n        const token = res.data?.token || res.token;\n        const item = res.data?.record || res.record || res.data?.item || res.item;\n        if (token) pm.collectionVariables.set('auth_token', token);\n        const id = item?.id;\n        if (id) {{\n            pm.collectionVariables.set('{id_var_key}', id);\n            pm.collectionVariables.set('user_id', id);\n        }}\n    }}\n}} catch (e) {{\n    console.warn('Auth script bypassed:', e);\n}}"
        );

        items.push(json!({
            "name": format!("📝 Register ({})", title_name),
            "event": [
                {
                    "listen": "test",
                    "script": {
                        "exec": auth_test_script.lines().map(String::from).collect::<Vec<_>>(),
                        "type": "text/javascript"
                    }
                }
            ],
            "request": {
                "method": "POST",
                "header": [
                    { "key": "Content-Type", "value": "application/json" }
                ],
                "body": {
                    "mode": "raw",
                    "raw": "{\n  \"email\": \"user@example.com\",\n  \"username\": \"user123\",\n  \"name\": \"Sample User\",\n  \"password\": \"Password123!@#\",\n  \"passwordConfirm\": \"Password123!@#\"\n}"
                },
                "url": {
                    "raw": format!("{{{{base_url}}}}/api/collections/{}/register", c_name)
                },
                "description": format!("Register a new account in auth collection '{}' and immediately receive a JWT auth_token.", c_name)
            }
        }));

        items.push(json!({
            "name": format!("🔐 Auth With Password / Login ({})", title_name),
            "event": [
                {
                    "listen": "test",
                    "script": {
                        "exec": auth_test_script.lines().map(String::from).collect::<Vec<_>>(),
                        "type": "text/javascript"
                    }
                }
            ],
            "request": {
                "method": "POST",
                "header": [
                    { "key": "Content-Type", "value": "application/json" }
                ],
                "body": {
                    "mode": "raw",
                    "raw": "{\n  \"identity\": \"user@example.com\",\n  \"password\": \"Password123!@#\"\n}"
                },
                "url": {
                    "raw": format!("{{{{base_url}}}}/api/collections/{}/auth-with-password", c_name)
                },
                "description": format!("Authenticate an auth record in collection '{}' with identity (email or username) and password. Automatically stores auth_token and ID variables.", c_name)
            }
        }));

        items.push(json!({
            "name": format!("🔑 Forgot Password ({})", title_name),
            "request": {
                "method": "POST",
                "header": [
                    { "key": "Content-Type", "value": "application/json" }
                ],
                "body": {
                    "mode": "raw",
                    "raw": "{\n  \"email\": \"user@example.com\"\n}"
                },
                "url": {
                    "raw": format!("{{{{base_url}}}}/api/collections/{}/forgot-password", c_name)
                },
                "description": format!("Request a password reset token for an account in '{}'.", c_name)
            }
        }));

        items.push(json!({
            "name": format!("🔄 Reset Password ({})", title_name),
            "request": {
                "method": "POST",
                "header": [
                    { "key": "Content-Type", "value": "application/json" }
                ],
                "body": {
                    "mode": "raw",
                    "raw": "{\n  \"token\": \"PASTE_RESET_TOKEN_HERE\",\n  \"password\": \"NewPassword123!@#\",\n  \"passwordConfirm\": \"NewPassword123!@#\"\n}"
                },
                "url": {
                    "raw": format!("{{{{base_url}}}}/api/collections/{}/reset-password", c_name)
                },
                "description": format!("Confirm password reset using the reset token for '{}'.", c_name)
            }
        }));

        items.push(json!({
            "name": format!("🔒 Change Password ({})", title_name),
            "request": {
                "method": "POST",
                "header": [
                    { "key": "Content-Type", "value": "application/json" }
                ],
                "body": {
                    "mode": "raw",
                    "raw": "{\n  \"oldPassword\": \"Password123!@#\",\n  \"newPassword\": \"UpdatedPass123!@#\",\n  \"passwordConfirm\": \"UpdatedPass123!@#\"\n}"
                },
                "url": {
                    "raw": format!("{{{{base_url}}}}/api/collections/{}/change-password", c_name)
                },
                "description": format!("Change password for the currently logged-in user in '{}'.", c_name)
            }
        }));

        items.push(json!({
            "name": format!("👤 Get Current User & Refresh Token ({})", title_name),
            "request": {
                "method": "GET",
                "header": [],
                "url": {
                    "raw": format!("{{{{base_url}}}}/api/collections/{}/me", c_name)
                },
                "description": format!("Get the currently authenticated user profile and refreshed JWT token in '{}'.", c_name)
            }
        }));
    }

    // 2. List & Paginate Records
    items.push(json!({
        "name": format!("List {} (Paginated)", title_name),
        "request": {
            "method": "GET",
            "header": [],
            "url": {
                "raw": format!("{{{{base_url}}}}/api/collections/{}/records?page=1&perPage=30&sort=-created", c_name)
            },
            "description": format!("Fetch paginated records for '{}' with sorting, filtering, and standard Padi Rust envelope.", c_name)
        }
    }));

    // 3. List All Records (Padi Rust convention 1: /records/all)
    items.push(json!({
        "name": format!("List All {} (Padi Rust records/all)", title_name),
        "request": {
            "method": "GET",
            "header": [],
            "url": {
                "raw": format!("{{{{base_url}}}}/api/collections/{}/records/all?sort_by=id&order=desc", c_name)
            },
            "description": format!("Retrieve all records for '{}' without pagination (Padi Rust standard).", c_name)
        }
    }));

    // 4. List All Records (Padi Rust convention 2: /api/{collection}/all)
    items.push(json!({
        "name": format!("List All {} (Direct /api/{}/all)", title_name, c_name),
        "request": {
            "method": "GET",
            "header": [],
            "url": {
                "raw": format!("{{{{base_url}}}}/api/{}/all?sort_by=id&order=desc", c_name)
            },
            "description": format!("Clean direct route matching Padi Rust standard (GET /api/{}/all).", c_name)
        }
    }));

    // 5. Get Record by ID
    items.push(json!({
        "name": format!("Get {} By ID", title_name),
        "request": {
            "method": "GET",
            "header": [],
            "url": {
                "raw": format!("{{{{base_url}}}}/api/collections/{}/records/{{{{{}}}}}", c_name, id_var_key)
            },
            "description": format!("Get a single record from '{}' by its 15-character unique ID.", c_name)
        }
    }));

    // 6. Create Record
    items.push(json!({
        "name": format!("Create {}", title_name),
        "event": [
            {
                "listen": "test",
                "script": {
                    "exec": record_id_test_script.lines().map(String::from).collect::<Vec<_>>(),
                    "type": "text/javascript"
                }
            }
        ],
        "request": {
            "method": "POST",
            "header": [
                { "key": "Content-Type", "value": "application/json" }
            ],
            "body": {
                "mode": "raw",
                "raw": create_body_raw
            },
            "url": {
                "raw": format!("{{{{base_url}}}}/api/collections/{}/records", c_name)
            },
            "description": format!("Insert a new record into '{}'. Automatically saves the created ID into variable '{{{}}}'.", c_name, id_var_key)
        }
    }));

    // 7. Update Record
    items.push(json!({
        "name": format!("Update {}", title_name),
        "request": {
            "method": "PATCH",
            "header": [
                { "key": "Content-Type", "value": "application/json" }
            ],
            "body": {
                "mode": "raw",
                "raw": update_body_raw
            },
            "url": {
                "raw": format!("{{{{base_url}}}}/api/collections/{}/records/{{{{{}}}}}", c_name, id_var_key)
            },
            "description": format!("Partially update an existing record in '{}'.", c_name)
        }
    }));

    // 8. Delete Record
    items.push(json!({
        "name": format!("Delete {}", title_name),
        "request": {
            "method": "DELETE",
            "header": [],
            "url": {
                "raw": format!("{{{{base_url}}}}/api/collections/{}/records/{{{{{}}}}}", c_name, id_var_key)
            },
            "description": format!("Delete an existing record in '{}' by ID.", c_name)
        }
    }));

    // 9. Export Records JSON
    items.push(json!({
        "name": format!("Export Records JSON ({})", title_name),
        "request": {
            "method": "GET",
            "header": [],
            "url": {
                "raw": format!("{{{{base_url}}}}/api/collections/{}/export", c_name)
            },
            "description": format!("Export all records from collection '{}' as raw JSON.", c_name)
        }
    }));

    // 10. Import Records JSON
    items.push(json!({
        "name": format!("Import Records JSON ({})", title_name),
        "request": {
            "method": "POST",
            "header": [
                { "key": "Content-Type", "value": "application/json" }
            ],
            "body": {
                "mode": "raw",
                "raw": format!("[\n  {}\n]", create_body_raw)
            },
            "url": {
                "raw": format!("{{{{base_url}}}}/api/collections/{}/import", c_name)
            },
            "description": format!("Bulk import an array of record objects into '{}'.", c_name)
        }
    }));

    // 11. Upload File Attachment
    items.push(json!({
        "name": format!("Upload File Attachment ({})", title_name),
        "request": {
            "method": "POST",
            "body": {
                "mode": "formdata",
                "formdata": [
                    {
                        "key": "file",
                        "type": "file",
                        "description": "Attachment file to upload"
                    }
                ]
            },
            "url": {
                "raw": format!("{{{{base_url}}}}/api/files/{}/{{{{{}}}}}", c_name, id_var_key)
            },
            "description": format!("Upload an attachment file to an existing '{}' record via multipart/form-data.", c_name)
        }
    }));

    // 12. Download File Attachment
    items.push(json!({
        "name": format!("Download File Attachment ({})", title_name),
        "request": {
            "method": "GET",
            "header": [],
            "url": {
                "raw": format!("{{{{base_url}}}}/api/files/{}/{{{{{}}}}}/FILENAME", c_name, id_var_key)
            },
            "description": format!("Download or view a stored file attachment from '{}'.", c_name)
        }
    }));

    items
}

/// Build Postman Collection v2.1.0 for a single collection
pub fn build_single_collection_json(col: &Collection, base_url: &str) -> Value {
    let singular = singular_name(&col.name);
    let title = pascal_case(&col.name);
    let id_var_key = format!("{}_id", singular);

    json!({
        "info": {
            "_postman_id": Uuid::new_v4().to_string(),
            "name": format!("{} API Collection (PadiBase)", title),
            "description": format!("REST Client API Collection for '{}' generated by PadiBase (compatible with Postman, Insomnia, Thunder Client, Bruno, and VS Code REST Client).", col.name),
            "schema": "https://schema.getpostman.com/json/collection/v2.1.0/collection.json"
        },
        "auth": {
            "type": "bearer",
            "bearer": [
                {
                    "key": "token",
                    "value": "{{auth_token}}",
                    "type": "string"
                }
            ]
        },
        "variable": [
            { "key": "base_url", "value": base_url },
            { "key": "auth_token", "value": "" },
            { "key": "record_id", "value": "1" },
            { "key": id_var_key, "value": "1" }
        ],
        "item": build_collection_items(col)
    })
}

/// Build system & admin folder items
fn build_system_and_admin_folders() -> Vec<Value> {
    let admin_login_test = "try {\n    if (pm.response.code === 200) {\n        const res = pm.response.json();\n        const token = res.data?.token || res.token;\n        if (token) {\n            pm.collectionVariables.set('auth_token', token);\n            pm.collectionVariables.set('admin_token', token);\n        }\n    }\n} catch (e) {\n    console.warn('Admin auth script bypassed:', e);\n}";

    vec![
        // Folder: System & Health
        json!({
            "name": "⚡ System & Health",
            "item": [
                {
                    "name": "Health Check",
                    "request": {
                        "method": "GET",
                        "header": [],
                        "url": { "raw": "{{base_url}}/api/health" },
                        "description": "Verify that PadiBase server and database engine are healthy and operational."
                    }
                },
                {
                    "name": "API Root & Summary",
                    "request": {
                        "method": "GET",
                        "header": [],
                        "url": { "raw": "{{base_url}}/api" },
                        "description": "Discover server identity, active database driver, and available core endpoints."
                    }
                },
                {
                    "name": "SSE Realtime Event Stream",
                    "request": {
                        "method": "GET",
                        "header": [
                            { "key": "Accept", "value": "text/event-stream" }
                        ],
                        "url": { "raw": "{{base_url}}/api/realtime" },
                        "description": "Subscribe to real-time Server-Sent Events (SSE) broadcast for create, update, and delete actions."
                    }
                }
            ]
        }),
        // Folder: Superuser & Admins
        json!({
            "name": "🔐 Admin Gateway & Settings",
            "item": [
                {
                    "name": "Check Has Admins",
                    "request": {
                        "method": "GET",
                        "header": [],
                        "url": { "raw": "{{base_url}}/api/admins/has-admins" },
                        "description": "Check if at least one Superuser exists in the system."
                    }
                },
                {
                    "name": "Admin Auth with Password (Login)",
                    "event": [
                        {
                            "listen": "test",
                            "script": {
                                "exec": admin_login_test.lines().map(String::from).collect::<Vec<_>>(),
                                "type": "text/javascript"
                            }
                        }
                    ],
                    "request": {
                        "method": "POST",
                        "header": [
                            { "key": "Content-Type", "value": "application/json" }
                        ],
                        "body": {
                            "mode": "raw",
                            "raw": "{\n  \"identity\": \"{{admin_email}}\",\n  \"password\": \"{{admin_password}}\"\n}"
                        },
                        "url": { "raw": "{{base_url}}/api/admins/auth-with-password" },
                        "description": "Authenticate Superuser with email and password. Automatically sets the 'auth_token' collection variable."
                    }
                },
                {
                    "name": "List Admins",
                    "request": {
                        "method": "GET",
                        "header": [],
                        "url": { "raw": "{{base_url}}/api/admins" },
                        "description": "List all superuser accounts (Requires Bearer Token)."
                    }
                },
                {
                    "name": "Create Superuser Admin",
                    "request": {
                        "method": "POST",
                        "header": [
                            { "key": "Content-Type", "value": "application/json" }
                        ],
                        "body": {
                            "mode": "raw",
                            "raw": "{\n  \"email\": \"new_admin@example.com\",\n  \"password\": \"Password_Admin_123!\"\n}"
                        },
                        "url": { "raw": "{{base_url}}/api/admins" },
                        "description": "Create a new Superuser (Requires Bearer Token or first initialization)."
                    }
                },
                {
                    "name": "Get System Settings",
                    "request": {
                        "method": "GET",
                        "header": [],
                        "url": { "raw": "{{base_url}}/api/settings" },
                        "description": "Get PadiBase system configuration & metadata (Requires Bearer Token)."
                    }
                },
                {
                    "name": "Update System Settings",
                    "request": {
                        "method": "PATCH",
                        "header": [
                            { "key": "Content-Type", "value": "application/json" }
                        ],
                        "body": {
                            "mode": "raw",
                            "raw": "{\n  \"meta\": {\n    \"appName\": \"PadiBase Custom App\",\n    \"appUrl\": \"http://127.0.0.1:8085\"\n  }\n}"
                        },
                        "url": { "raw": "{{base_url}}/api/settings" },
                        "description": "Update system settings (Requires Bearer Token)."
                    }
                }
            ]
        }),
        // Folder: Collections Schema Management
        json!({
            "name": "📦 Collections Schema Management",
            "item": [
                {
                    "name": "List All Collections",
                    "request": {
                        "method": "GET",
                        "header": [],
                        "url": { "raw": "{{base_url}}/api/collections" },
                        "description": "List all collections and their schema definitions."
                    }
                },
                {
                    "name": "Sync Collections With Database Tables",
                    "request": {
                        "method": "POST",
                        "header": [],
                        "url": { "raw": "{{base_url}}/api/collections/sync" },
                        "description": "Introspect database tables and automatically synchronize schema definitions."
                    }
                },
                {
                    "name": "Create Collection",
                    "request": {
                        "method": "POST",
                        "header": [
                            { "key": "Content-Type", "value": "application/json" }
                        ],
                        "body": {
                            "mode": "raw",
                            "raw": "{\n  \"name\": \"posts\",\n  \"type\": \"base\",\n  \"realtime\": true,\n  \"schema\": [\n    {\n      \"name\": \"title\",\n      \"type\": \"text\",\n      \"required\": true\n    },\n    {\n      \"name\": \"content\",\n      \"type\": \"text\",\n      \"required\": false\n    },\n    {\n      \"name\": \"published\",\n      \"type\": \"bool\",\n      \"required\": false\n    }\n  ]\n}"
                        },
                        "url": { "raw": "{{base_url}}/api/collections" },
                        "description": "Create a new Base or Auth collection and automatically generate SQL table."
                    }
                },
                {
                    "name": "Get Collection Schema By Name",
                    "request": {
                        "method": "GET",
                        "header": [],
                        "url": { "raw": "{{base_url}}/api/collections/users" },
                        "description": "Get detailed schema and rule definitions for a collection."
                    }
                }
            ]
        }),
        // Folder: Backups
        json!({
            "name": "💾 System Backups",
            "item": [
                {
                    "name": "List Backups",
                    "request": {
                        "method": "GET",
                        "header": [],
                        "url": { "raw": "{{base_url}}/api/backups" },
                        "description": "List available online backup snapshot zip files."
                    }
                },
                {
                    "name": "Create Online Backup Snapshot",
                    "request": {
                        "method": "POST",
                        "header": [],
                        "url": { "raw": "{{base_url}}/api/backups" },
                        "description": "Create an atomic online backup snapshot of database and files."
                    }
                }
            ]
        }),
        // Folder: Transactions & Custom SQL
        json!({
            "name": "🔄 Database Transactions & Custom SQL",
            "item": [
                {
                    "name": "Execute Atomic Batch Transaction",
                    "request": {
                        "method": "POST",
                        "header": [
                            { "key": "Content-Type", "value": "application/json" }
                        ],
                        "body": {
                            "mode": "raw",
                            "raw": "{\n  \"operations\": [\n    {\n      \"action\": \"create\",\n      \"collection\": \"posts\",\n      \"data\": { \"title\": \"Atomic Post 1\" }\n    },\n    {\n      \"action\": \"create\",\n      \"collection\": \"posts\",\n      \"data\": { \"title\": \"Atomic Post 2\" }\n    }\n  ]\n}"
                        },
                        "url": { "raw": "{{base_url}}/api/transactions" },
                        "description": "Execute multiple create, update, delete, or admin SQL operations inside a single atomic database transaction (all-or-nothing rollback on error)."
                    }
                },
                {
                    "name": "Execute Custom Raw SQL Query (Admin)",
                    "request": {
                        "method": "POST",
                        "header": [
                            { "key": "Content-Type", "value": "application/json" }
                        ],
                        "body": {
                            "mode": "raw",
                            "raw": "{\n  \"sql\": \"SELECT id, email, role FROM `users` WHERE `role` = ? LIMIT 10\",\n  \"params\": [\"user\"],\n  \"transaction\": false\n}"
                        },
                        "url": { "raw": "{{base_url}}/api/sql" },
                        "description": "Run custom parameterized SQL queries (JOIN, GROUP BY, aggregations) as Superuser/Admin."
                    }
                },
                {
                    "name": "Custom Atomic Transfer Example",
                    "request": {
                        "method": "POST",
                        "header": [
                            { "key": "Content-Type", "value": "application/json" }
                        ],
                        "body": {
                            "mode": "raw",
                            "raw": "{\n  \"collection\": \"wallets\",\n  \"from_id\": \"record_1\",\n  \"to_id\": \"record_2\",\n  \"field\": \"balance\",\n  \"amount\": 50\n}"
                        },
                        "url": { "raw": "{{base_url}}/api/custom/transfer" },
                        "description": "Example custom transactional endpoint that atomically transfers balance/stock between two records."
                    }
                }
            ]
        })
    ]
}

/// Build complete Postman Collection v2.1.0 for all collections and core system endpoints
pub fn build_all_collections_json(cols: &[Collection], base_url: &str) -> Value {
    let mut root_items = build_system_and_admin_folders();

    // Add a folder for each user collection
    for col in cols {
        let folder_title = format!("📂 {}", pascal_case(&col.name));
        let col_items = build_collection_items(col);
        root_items.push(json!({
            "name": folder_title,
            "description": format!("CRUD, Padi Rust endpoints, Realtime, Import, and Export for collection '{}'", col.name),
            "item": col_items
        }));
    }

    json!({
        "info": {
            "_postman_id": Uuid::new_v4().to_string(),
            "name": "PadiBase API Collection (Full)",
            "description": "Complete REST Client API Collection for PadiBase Backend-as-a-Service, matching Padi Rust standards. Compatible with Postman, Insomnia, Thunder Client, Bruno, and VS Code REST Client.",
            "schema": "https://schema.getpostman.com/json/collection/v2.1.0/collection.json"
        },
        "auth": {
            "type": "bearer",
            "bearer": [
                {
                    "key": "token",
                    "value": "{{auth_token}}",
                    "type": "string"
                }
            ]
        },
        "variable": [
            { "key": "base_url", "value": base_url },
            { "key": "auth_token", "value": "" },
            { "key": "admin_token", "value": "" },
            { "key": "admin_email", "value": "admin@example.com" },
            { "key": "admin_password", "value": "password_rahasia_123" },
            { "key": "record_id", "value": "1" }
        ],
        "item": root_items
    })
}
