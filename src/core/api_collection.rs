use serde_json::{json, Map, Value};
use std::collections::{BTreeMap, HashSet};
use uuid::Uuid;

use super::schema::{Collection, CollectionType, CustomEndpoint, FieldType, SchemaField};

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
    if let Some(stem) = lower.strip_suffix("ies") {
        if !stem.is_empty() {
            return format!("{}y", stem);
        }
    }
    if (lower.ends_with("shes")
        || lower.ends_with("ches")
        || lower.ends_with("xes")
        || lower.ends_with("sses"))
        && lower.len() > 2
    {
        if let Some(stem) = lower.strip_suffix("es") {
            if !stem.is_empty() {
                return stem.to_string();
            }
        }
    }
    if !lower.ends_with("ss") {
        if let Some(stem) = lower.strip_suffix('s') {
            if !stem.is_empty() {
                return stem.to_string();
            }
        }
    }
    lower
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
            } else if name_lower.ends_with("_url")
                || name_lower.contains("avatar")
                || name_lower.contains("photo")
            {
                json!("https://example.com/avatar.jpg")
            } else {
                json!(format!("Sample {}", pascal_case(&field.name)))
            }
        }
    }
}

/// Generate sample create and update JSON objects for a collection
pub fn generate_sample_payloads(col: &Collection) -> (Value, Value) {
    let mut create_map = Map::new();
    let mut update_map = Map::new();

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

    if create_map.is_empty() {
        create_map.insert(
            "title".to_string(),
            json!(format!("Sample {}", pascal_case(&col.name))),
        );
        create_map.insert("description".to_string(), json!("PadiBase sample record"));
    }
    if update_map.is_empty() {
        update_map.insert(
            "title".to_string(),
            json!(format!("Updated {}", pascal_case(&col.name))),
        );
    }

    (Value::Object(create_map), Value::Object(update_map))
}

#[derive(Clone)]
struct QueryParamSpec {
    key: String,
    value: String,
    disabled: bool,
    description: String,
}

#[derive(Clone)]
struct PathParamSpec {
    name: String,
    example: String,
    description: String,
}

#[derive(Clone)]
struct FormFieldSpec {
    key: String,
    is_file: bool,
    description: String,
}

#[derive(Clone)]
enum BodySpec {
    None,
    Json(Value),
    FormData(Vec<FormFieldSpec>),
}

#[derive(Clone)]
struct EndpointSpec {
    id: String,
    name: String,
    method: String,
    path_template: String,
    path_params: Vec<PathParamSpec>,
    query: Vec<QueryParamSpec>,
    headers: Vec<(String, String)>,
    body: BodySpec,
    auth_required: bool,
    description: String,
    test_script: Option<String>,
}

struct FolderSpec {
    id: String,
    name: String,
    tag: String,
    description: String,
    endpoints: Vec<EndpointSpec>,
}

struct CanonicalSpec {
    title: String,
    description: String,
    base_url: String,
    is_single: bool,
    variables: Vec<(String, String)>,
    folders: Vec<FolderSpec>,
}

fn replace_path_vars_postman(path_template: &str) -> String {
    let mut out = String::with_capacity(path_template.len() + 8);
    let mut chars = path_template.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '{' {
            let mut name = String::new();
            while let Some(&nc) = chars.peek() {
                chars.next();
                if nc == '}' {
                    break;
                }
                name.push(nc);
            }
            out.push_str("{{");
            out.push_str(&name);
            out.push_str("}}");
        } else {
            out.push(c);
        }
    }
    out
}

fn replace_path_vars_insomnia(path_template: &str) -> String {
    let mut out = String::with_capacity(path_template.len() + 16);
    let mut chars = path_template.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '{' {
            let mut name = String::new();
            while let Some(&nc) = chars.peek() {
                chars.next();
                if nc == '}' {
                    break;
                }
                name.push(nc);
            }
            out.push_str("{{ _.");
            out.push_str(&name);
            out.push_str(" }}");
        } else {
            out.push(c);
        }
    }
    out
}

fn build_postman_url(path_template: &str, query: &[QueryParamSpec]) -> Value {
    let postman_path = replace_path_vars_postman(path_template);
    let segments: Vec<&str> = postman_path
        .trim_start_matches('/')
        .split('/')
        .filter(|s| !s.is_empty())
        .collect();

    let active_query: Vec<String> = query
        .iter()
        .filter(|q| !q.disabled)
        .map(|q| format!("{}={}", q.key, q.value))
        .collect();

    let raw = if active_query.is_empty() {
        format!("{{{{base_url}}}}{}", postman_path)
    } else {
        format!("{{{{base_url}}}}{}?{}", postman_path, active_query.join("&"))
    };

    let mut obj = Map::new();
    obj.insert("raw".to_string(), json!(raw));
    obj.insert("host".to_string(), json!(["{{base_url}}"]));
    obj.insert("path".to_string(), json!(segments));

    if !query.is_empty() {
        let q_items: Vec<Value> = query
            .iter()
            .map(|q| {
                let mut q_obj = Map::new();
                q_obj.insert("key".to_string(), json!(q.key));
                q_obj.insert("value".to_string(), json!(q.value));
                if q.disabled {
                    q_obj.insert("disabled".to_string(), json!(true));
                }
                if !q.description.is_empty() {
                    q_obj.insert("description".to_string(), json!(q.description));
                }
                Value::Object(q_obj)
            })
            .collect();
        obj.insert("query".to_string(), Value::Array(q_items));
    }

    Value::Object(obj)
}

/// Resolve custom endpoint path so it always starts with `/`.
pub fn resolve_custom_endpoint_path(raw_path: &str) -> String {
    let clean = raw_path.trim().trim_matches('/');
    if clean.is_empty() {
        return "/api/custom-endpoint".to_string();
    }
    let lower = clean.to_lowercase();
    if lower.starts_with("api/") {
        format!("/{}", clean)
    } else if lower.starts_with("custom/") {
        format!("/api/{}", clean)
    } else {
        format!("/api/{}", clean)
    }
}

fn strip_sql_comments_and_literals(sql: &str) -> String {
    let chars: Vec<char> = sql.chars().collect();
    let len = chars.len();
    let mut out = String::with_capacity(len);
    let mut i = 0;
    let mut in_single = false;

    while i < len {
        let c = chars[i];
        let next = chars.get(i + 1).copied();

        if !in_single && c == '-' && next == Some('-') {
            while i < len && chars[i] != '\n' {
                i += 1;
            }
            out.push(' ');
            continue;
        }
        if !in_single && c == '/' && next == Some('*') {
            i += 2;
            while i < len && !(chars[i] == '*' && chars.get(i + 1).copied() == Some('/')) {
                i += 1;
            }
            i = (i + 2).min(len);
            out.push(' ');
            continue;
        }
        if c == '\'' {
            if in_single && next == Some('\'') {
                i += 2;
                continue;
            }
            in_single = !in_single;
            out.push(' ');
            i += 1;
            continue;
        }
        if !in_single {
            out.push(c);
        }
        i += 1;
    }

    out
}

fn is_system_sql_param(name: &str) -> bool {
    matches!(
        name.to_lowercase().as_str(),
        "auth_id"
            | "auth_email"
            | "auth_role"
            | "request.auth.id"
            | "request.auth.email"
            | "request.auth.role"
            | "last_insert_id"
            | "last_id"
    )
}

fn sample_value_for_param_name(name: &str) -> Value {
    let lower = name.to_lowercase();
    if lower == "limit" || lower == "per_page" || lower == "perpage" {
        json!(10)
    } else if lower == "page" {
        json!(1)
    } else if lower == "offset" {
        json!(0)
    } else if lower == "id" || lower.ends_with("_id") {
        json!(1)
    } else if lower.contains("email") {
        json!("user@example.com")
    } else if lower.starts_with("min_")
        || lower.starts_with("max_")
        || lower.contains("amount")
        || lower.contains("price")
        || lower.contains("qty")
    {
        json!(100)
    } else {
        json!(format!("sample_{}", name))
    }
}

/// Extract path parameters (`:id`, `{id}`) and SQL named parameters (`:param`, `{{param}}`)
/// while ignoring Postgres casts (`::type`) and string literals.
pub fn extract_custom_endpoint_params(resolved_path: &str, sql: &str) -> (Vec<String>, Vec<String>) {
    let mut path_params = Vec::new();
    let mut path_set = HashSet::new();

    // Extract path params from `:param` or `{param}`
    let path_chars: Vec<char> = resolved_path.chars().collect();
    let mut i = 0;
    while i < path_chars.len() {
        let c = path_chars[i];
        if c == ':' && i + 1 < path_chars.len() && (path_chars[i + 1].is_ascii_alphabetic() || path_chars[i + 1] == '_') {
            let mut end = i + 1;
            while end < path_chars.len() && (path_chars[end].is_ascii_alphanumeric() || path_chars[end] == '_') {
                end += 1;
            }
            let name: String = path_chars[i + 1..end].iter().collect();
            if path_set.insert(name.clone()) {
                path_params.push(name);
            }
            i = end;
            continue;
        }
        if c == '{' {
            let mut end = i + 1;
            while end < path_chars.len() && path_chars[end] != '}' {
                end += 1;
            }
            if end < path_chars.len() {
                let name: String = path_chars[i + 1..end].iter().collect();
                let clean = name.trim().to_string();
                if !clean.is_empty() && path_set.insert(clean.clone()) {
                    path_params.push(clean);
                }
                i = end + 1;
                continue;
            }
        }
        i += 1;
    }

    let cleaned_sql = strip_sql_comments_and_literals(sql);
    let sql_chars: Vec<char> = cleaned_sql.chars().collect();
    let mut sql_params = Vec::new();
    let mut sql_set = HashSet::new();
    let mut j = 0;

    while j < sql_chars.len() {
        let c = sql_chars[j];
        // Skip `::type` Postgres casts
        if c == ':' && j + 1 < sql_chars.len() && sql_chars[j + 1] == ':' {
            j += 2;
            continue;
        }
        if c == ':' && j + 1 < sql_chars.len() && (sql_chars[j + 1].is_ascii_alphabetic() || sql_chars[j + 1] == '_') {
            let prev_is_colon = j > 0 && sql_chars[j - 1] == ':';
            if !prev_is_colon {
                let mut end = j + 1;
                while end < sql_chars.len()
                    && (sql_chars[end].is_ascii_alphanumeric() || sql_chars[end] == '_' || sql_chars[end] == '.')
                {
                    end += 1;
                }
                let name: String = sql_chars[j + 1..end].iter().collect();
                if !is_system_sql_param(&name) && !path_set.contains(&name) && sql_set.insert(name.clone()) {
                    sql_params.push(name);
                }
                j = end;
                continue;
            }
        }
        if c == '{' && j + 1 < sql_chars.len() && sql_chars[j + 1] == '{' {
            let mut end = j + 2;
            while end + 1 < sql_chars.len() && !(sql_chars[end] == '}' && sql_chars[end + 1] == '}') {
                end += 1;
            }
            if end + 1 < sql_chars.len() {
                let name: String = sql_chars[j + 2..end].iter().collect();
                let clean = name.trim().to_string();
                if !clean.is_empty()
                    && !is_system_sql_param(&clean)
                    && !path_set.contains(&clean)
                    && sql_set.insert(clean.clone())
                {
                    sql_params.push(clean);
                }
                j = end + 2;
                continue;
            }
        }
        j += 1;
    }

    (path_params, sql_params)
}

fn normalize_path_template(resolved_path: &str) -> String {
    let chars: Vec<char> = resolved_path.chars().collect();
    let mut out = String::with_capacity(chars.len());
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == ':' && i + 1 < chars.len() && (chars[i + 1].is_ascii_alphabetic() || chars[i + 1] == '_') {
            let mut end = i + 1;
            while end < chars.len() && (chars[end].is_ascii_alphanumeric() || chars[end] == '_') {
                end += 1;
            }
            let name: String = chars[i + 1..end].iter().collect();
            out.push('{');
            out.push_str(&name);
            out.push('}');
            i = end;
        } else {
            out.push(chars[i]);
            i += 1;
        }
    }
    out
}

fn build_table_endpoints(col: &Collection) -> Vec<EndpointSpec> {
    let c_name = &col.name;
    let singular = singular_name(c_name);
    let title_name = pascal_case(c_name);
    let id_var_key = format!("{}_id", singular);
    let (sample_create, sample_update) = generate_sample_payloads(col);

    let record_id_test_script = format!(
        "try {{\n    if (pm.response.code === 201 || pm.response.code === 200) {{\n        const res = pm.response.json();\n        const item = res.item || res.data?.item || res.data || res;\n        const id = item?.id || res.id;\n        if (id) {{\n            pm.collectionVariables.set('{id_var_key}', id);\n            pm.collectionVariables.set('record_id', id);\n        }}\n    }}\n}} catch (e) {{\n    console.warn('Script bypassed:', e);\n}}"
    );

    let mut items = Vec::new();

    if col.collection_type == CollectionType::Auth {
        let auth_test_script = format!(
            "try {{\n    if (pm.response.code === 200 || pm.response.code === 201) {{\n        const res = pm.response.json();\n        const item = res.item || res.data?.item || res.data || res;\n        const token = item?.token || res.data?.token || res.token;\n        const rec = item?.record || res.data?.record || res.record || item;\n        if (token) pm.collectionVariables.set('auth_token', token);\n        const id = rec?.id;\n        if (id) {{\n            pm.collectionVariables.set('{id_var_key}', id);\n            pm.collectionVariables.set('user_id', id);\n        }}\n    }}\n}} catch (e) {{\n    console.warn('Auth script bypassed:', e);\n}}"
        );

        items.push(EndpointSpec {
            id: format!("{}_register", c_name),
            name: format!("📝 Register ({})", title_name),
            method: "POST".to_string(),
            path_template: format!("/api/{}/register", c_name),
            path_params: vec![],
            query: vec![],
            headers: vec![],
            body: BodySpec::Json(json!({
                "email": "user@example.com",
                "username": "user123",
                "name": "Sample User",
                "password": "Password123!@#",
                "passwordConfirm": "Password123!@#"
            })),
            auth_required: false,
            description: format!("Register a new account in auth table '{}' and immediately receive a JWT auth_token.", c_name),
            test_script: Some(auth_test_script.clone()),
        });

        items.push(EndpointSpec {
            id: format!("{}_login", c_name),
            name: format!("🔐 Auth With Password / Login ({})", title_name),
            method: "POST".to_string(),
            path_template: format!("/api/{}/auth-with-password", c_name),
            path_params: vec![],
            query: vec![],
            headers: vec![],
            body: BodySpec::Json(json!({
                "identity": "user@example.com",
                "password": "Password123!@#"
            })),
            auth_required: false,
            description: format!("Authenticate an auth record in table '{}' with identity (email or username) and password.", c_name),
            test_script: Some(auth_test_script),
        });

        items.push(EndpointSpec {
            id: format!("{}_me", c_name),
            name: format!("👤 Get Current User & Refresh Token ({})", title_name),
            method: "GET".to_string(),
            path_template: format!("/api/{}/me", c_name),
            path_params: vec![],
            query: vec![],
            headers: vec![],
            body: BodySpec::None,
            auth_required: true,
            description: format!("Get the currently authenticated user profile and refreshed JWT token in '{}'.", c_name),
            test_script: None,
        });

        items.push(EndpointSpec {
            id: format!("{}_me_patch", c_name),
            name: format!("✏️ Update Current Profile ({})", title_name),
            method: "PATCH".to_string(),
            path_template: format!("/api/{}/me", c_name),
            path_params: vec![],
            query: vec![],
            headers: vec![],
            body: BodySpec::Json(json!({ "name": "Updated User Name" })),
            auth_required: true,
            description: format!("Update profile fields for the currently authenticated user in '{}'.", c_name),
            test_script: None,
        });

        items.push(EndpointSpec {
            id: format!("{}_logout", c_name),
            name: format!("🚪 Logout ({})", title_name),
            method: "POST".to_string(),
            path_template: format!("/api/{}/logout", c_name),
            path_params: vec![],
            query: vec![],
            headers: vec![],
            body: BodySpec::None,
            auth_required: true,
            description: format!("Revoke active session token for the logged-in user in '{}'.", c_name),
            test_script: None,
        });

        items.push(EndpointSpec {
            id: format!("{}_forgot", c_name),
            name: format!("🔑 Forgot Password ({})", title_name),
            method: "POST".to_string(),
            path_template: format!("/api/{}/forgot-password", c_name),
            path_params: vec![],
            query: vec![],
            headers: vec![],
            body: BodySpec::Json(json!({ "email": "user@example.com" })),
            auth_required: false,
            description: format!("Request a password reset token for an account in '{}'.", c_name),
            test_script: None,
        });

        items.push(EndpointSpec {
            id: format!("{}_reset", c_name),
            name: format!("🔄 Reset Password ({})", title_name),
            method: "POST".to_string(),
            path_template: format!("/api/{}/reset-password", c_name),
            path_params: vec![],
            query: vec![],
            headers: vec![],
            body: BodySpec::Json(json!({
                "token": "PASTE_RESET_TOKEN_HERE",
                "password": "NewPassword123!@#",
                "passwordConfirm": "NewPassword123!@#"
            })),
            auth_required: false,
            description: format!("Confirm password reset using the reset token for '{}'.", c_name),
            test_script: None,
        });

        items.push(EndpointSpec {
            id: format!("{}_change_pw", c_name),
            name: format!("🔒 Change Password ({})", title_name),
            method: "POST".to_string(),
            path_template: format!("/api/{}/change-password", c_name),
            path_params: vec![],
            query: vec![],
            headers: vec![],
            body: BodySpec::Json(json!({
                "oldPassword": "Password123!@#",
                "newPassword": "UpdatedPass123!@#",
                "passwordConfirm": "UpdatedPass123!@#"
            })),
            auth_required: true,
            description: format!("Change password for the currently logged-in user in '{}'.", c_name),
            test_script: None,
        });
    }

    let is_public = |rule: Option<&String>, default_public: bool| -> bool {
        match rule {
            Some(r) => r.trim().eq_ignore_ascii_case("public") || r.trim() == "*",
            None => default_public,
        }
    };

    items.push(EndpointSpec {
        id: format!("{}_list", c_name),
        name: format!("List {} (Paginated)", title_name),
        method: "GET".to_string(),
        path_template: format!("/api/{}", c_name),
        path_params: vec![],
        query: vec![
            QueryParamSpec { key: "page".to_string(), value: "1".to_string(), disabled: false, description: "Page number".to_string() },
            QueryParamSpec { key: "perPage".to_string(), value: "30".to_string(), disabled: false, description: "Records per page".to_string() },
            QueryParamSpec { key: "sort".to_string(), value: "-created".to_string(), disabled: false, description: "Sort order".to_string() },
            QueryParamSpec { key: "filter".to_string(), value: String::new(), disabled: true, description: "Optional filter expression".to_string() },
            QueryParamSpec { key: "expand".to_string(), value: String::new(), disabled: true, description: "Optional relation expansion".to_string() },
        ],
        headers: vec![],
        body: BodySpec::None,
        auth_required: !is_public(col.list_rule.as_ref(), true),
        description: format!("Fetch paginated records for '{}' with sorting, filtering, and standard Padi Rust envelope.", c_name),
        test_script: None,
    });

    items.push(EndpointSpec {
        id: format!("{}_all", c_name),
        name: format!("List All {}", title_name),
        method: "GET".to_string(),
        path_template: format!("/api/{}/all", c_name),
        path_params: vec![],
        query: vec![
            QueryParamSpec { key: "sort_by".to_string(), value: "id".to_string(), disabled: false, description: "Column to sort by".to_string() },
            QueryParamSpec { key: "order".to_string(), value: "desc".to_string(), disabled: false, description: "asc or desc".to_string() },
            QueryParamSpec { key: "filter".to_string(), value: String::new(), disabled: true, description: "Optional filter expression".to_string() },
        ],
        headers: vec![],
        body: BodySpec::None,
        auth_required: !is_public(col.all_rule.as_ref(), true),
        description: format!("Clean direct route to retrieve all records for '{}' without pagination (GET /api/{}/all).", c_name, c_name),
        test_script: None,
    });

    items.push(EndpointSpec {
        id: format!("{}_count", c_name),
        name: format!("Count {}", title_name),
        method: "GET".to_string(),
        path_template: format!("/api/{}/count", c_name),
        path_params: vec![],
        query: vec![
            QueryParamSpec { key: "filter".to_string(), value: String::new(), disabled: true, description: "Optional filter expression".to_string() },
        ],
        headers: vec![],
        body: BodySpec::None,
        auth_required: !is_public(col.list_rule.as_ref(), true),
        description: format!("Return total record count for '{}'.", c_name),
        test_script: None,
    });

    items.push(EndpointSpec {
        id: format!("{}_first", c_name),
        name: format!("Get First {}", title_name),
        method: "GET".to_string(),
        path_template: format!("/api/{}/first", c_name),
        path_params: vec![],
        query: vec![
            QueryParamSpec { key: "filter".to_string(), value: String::new(), disabled: true, description: "Optional filter expression".to_string() },
            QueryParamSpec { key: "sort".to_string(), value: "-created".to_string(), disabled: true, description: "Optional sort order".to_string() },
        ],
        headers: vec![],
        body: BodySpec::None,
        auth_required: !is_public(col.view_rule.as_ref(), true),
        description: format!("Retrieve the first matching record from '{}'.", c_name),
        test_script: None,
    });

    items.push(EndpointSpec {
        id: format!("{}_get_one", c_name),
        name: format!("Get {} By ID", title_name),
        method: "GET".to_string(),
        path_template: format!("/api/{}/{{{}}}", c_name, id_var_key),
        path_params: vec![PathParamSpec {
            name: id_var_key.clone(),
            example: "1".to_string(),
            description: format!("Record ID in '{}'", c_name),
        }],
        query: vec![],
        headers: vec![],
        body: BodySpec::None,
        auth_required: !is_public(col.view_rule.as_ref(), true),
        description: format!("Get a single record from '{}' by its unique ID.", c_name),
        test_script: None,
    });

    items.push(EndpointSpec {
        id: format!("{}_create", c_name),
        name: format!("Create {}", title_name),
        method: "POST".to_string(),
        path_template: format!("/api/{}", c_name),
        path_params: vec![],
        query: vec![],
        headers: vec![],
        body: BodySpec::Json(sample_create.clone()),
        auth_required: !is_public(col.create_rule.as_ref(), false),
        description: format!("Insert a new record into '{}' (POST /api/{}). Automatically saves the created ID into variable '{{{}}}'.", c_name, c_name, id_var_key),
        test_script: Some(record_id_test_script),
    });

    items.push(EndpointSpec {
        id: format!("{}_create_batch", c_name),
        name: format!("Batch Create {}", title_name),
        method: "POST".to_string(),
        path_template: format!("/api/{}/create-batch", c_name),
        path_params: vec![],
        query: vec![],
        headers: vec![],
        body: BodySpec::Json(json!({ "records": [sample_create.clone()] })),
        auth_required: !is_public(col.create_rule.as_ref(), false),
        description: format!("Insert multiple records into '{}' atomically.", c_name),
        test_script: None,
    });

    items.push(EndpointSpec {
        id: format!("{}_update", c_name),
        name: format!("Update {}", title_name),
        method: "PATCH".to_string(),
        path_template: format!("/api/{}/{{{}}}", c_name, id_var_key),
        path_params: vec![PathParamSpec {
            name: id_var_key.clone(),
            example: "1".to_string(),
            description: format!("Record ID in '{}'", c_name),
        }],
        query: vec![],
        headers: vec![],
        body: BodySpec::Json(sample_update),
        auth_required: !is_public(col.update_rule.as_ref(), false),
        description: format!("Partially update an existing record in '{}' by ID.", c_name),
        test_script: None,
    });

    items.push(EndpointSpec {
        id: format!("{}_delete", c_name),
        name: format!("Delete {}", title_name),
        method: "DELETE".to_string(),
        path_template: format!("/api/{}/{{{}}}", c_name, id_var_key),
        path_params: vec![PathParamSpec {
            name: id_var_key.clone(),
            example: "1".to_string(),
            description: format!("Record ID in '{}'", c_name),
        }],
        query: vec![],
        headers: vec![],
        body: BodySpec::None,
        auth_required: !is_public(col.delete_rule.as_ref(), false),
        description: format!("Delete an existing record in '{}' by ID.", c_name),
        test_script: None,
    });

    items.push(EndpointSpec {
        id: format!("{}_delete_batch", c_name),
        name: format!("Batch Delete {}", title_name),
        method: "POST".to_string(),
        path_template: format!("/api/{}/delete-batch", c_name),
        path_params: vec![],
        query: vec![],
        headers: vec![],
        body: BodySpec::Json(json!({ "ids": [format!("{{{{{}}}}}", id_var_key)] })),
        auth_required: !is_public(col.delete_rule.as_ref(), false),
        description: format!("Delete multiple records from '{}' by ID list.", c_name),
        test_script: None,
    });

    items.push(EndpointSpec {
        id: format!("{}_export", c_name),
        name: format!("Export Records JSON ({})", title_name),
        method: "GET".to_string(),
        path_template: format!("/api/{}/export", c_name),
        path_params: vec![],
        query: vec![],
        headers: vec![],
        body: BodySpec::None,
        auth_required: true,
        description: format!("Export all records from table '{}' as raw JSON.", c_name),
        test_script: None,
    });

    items.push(EndpointSpec {
        id: format!("{}_import", c_name),
        name: format!("Import Records JSON ({})", title_name),
        method: "POST".to_string(),
        path_template: format!("/api/{}/import", c_name),
        path_params: vec![],
        query: vec![],
        headers: vec![],
        body: BodySpec::Json(json!([sample_create])),
        auth_required: true,
        description: format!("Bulk import an array of record objects into '{}'.", c_name),
        test_script: None,
    });

    if let Some(ff) = col.schema.iter().find(|f| f.field_type == FieldType::File) {
        let ff_name = &ff.name;
        items.push(EndpointSpec {
            id: format!("{}_upload_file", c_name),
            name: format!("Upload File Attachment ({})", title_name),
            method: "POST".to_string(),
            path_template: format!("/api/files/{}/{{{}}}", c_name, id_var_key),
            path_params: vec![PathParamSpec {
                name: id_var_key.clone(),
                example: "1".to_string(),
                description: format!("Record ID in '{}'", c_name),
            }],
            query: vec![],
            headers: vec![],
            body: BodySpec::FormData(vec![FormFieldSpec {
                key: ff_name.clone(),
                is_file: true,
                description: format!("Attachment file to upload into column '{}'", ff_name),
            }]),
            auth_required: true,
            description: format!("Upload an attachment file to an existing '{}' record via multipart/form-data into column '{}'.", c_name, ff_name),
            test_script: None,
        });

        items.push(EndpointSpec {
            id: format!("{}_download_file", c_name),
            name: format!("Download File Attachment ({})", title_name),
            method: "GET".to_string(),
            path_template: format!("/api/files/{}/{{{}}}/{{filename}}", c_name, id_var_key),
            path_params: vec![
                PathParamSpec {
                    name: id_var_key,
                    example: "1".to_string(),
                    description: format!("Record ID in '{}'", c_name),
                },
                PathParamSpec {
                    name: "filename".to_string(),
                    example: "sample.png".to_string(),
                    description: "Stored attachment filename".to_string(),
                },
            ],
            query: vec![],
            headers: vec![],
            body: BodySpec::None,
            auth_required: false,
            description: format!("Download or view a stored file attachment from '{}'.", c_name),
            test_script: None,
        });
    }

    items
}

fn build_system_and_admin_folders(cols: &[Collection]) -> Vec<FolderSpec> {
    let sample_table = cols
        .first()
        .map(|c| c.name.as_str())
        .unwrap_or("users")
        .to_string();

    let admin_login_test = "try {\n    if (pm.response.code === 200) {\n        const res = pm.response.json();\n        const item = res.item || res.data || res;\n        const token = item?.token || res.data?.token || res.token;\n        if (token) {\n            pm.collectionVariables.set('auth_token', token);\n            pm.collectionVariables.set('admin_token', token);\n        }\n    }\n} catch (e) {\n    console.warn('Admin auth script bypassed:', e);\n}".to_string();

    vec![
        FolderSpec {
            id: "folder_system_health".to_string(),
            name: "⚡ System & Health".to_string(),
            tag: "System & Health".to_string(),
            description: "Core server health, API discovery, and SSE realtime stream endpoints.".to_string(),
            endpoints: vec![
                EndpointSpec {
                    id: "sys_health".to_string(),
                    name: "Health Check".to_string(),
                    method: "GET".to_string(),
                    path_template: "/api/health".to_string(),
                    path_params: vec![],
                    query: vec![],
                    headers: vec![],
                    body: BodySpec::None,
                    auth_required: false,
                    description: "Verify that PadiBase server and database engine are healthy and operational.".to_string(),
                    test_script: None,
                },
                EndpointSpec {
                    id: "sys_root".to_string(),
                    name: "API Root & Summary".to_string(),
                    method: "GET".to_string(),
                    path_template: "/api".to_string(),
                    path_params: vec![],
                    query: vec![],
                    headers: vec![],
                    body: BodySpec::None,
                    auth_required: false,
                    description: "Discover server identity, active database driver, and available core endpoints.".to_string(),
                    test_script: None,
                },
                EndpointSpec {
                    id: "sys_realtime".to_string(),
                    name: "SSE Realtime Event Stream".to_string(),
                    method: "GET".to_string(),
                    path_template: "/api/realtime".to_string(),
                    path_params: vec![],
                    query: vec![],
                    headers: vec![("Accept".to_string(), "text/event-stream".to_string())],
                    body: BodySpec::None,
                    auth_required: false,
                    description: "Subscribe to real-time Server-Sent Events (SSE) broadcast for create, update, and delete actions.".to_string(),
                    test_script: None,
                },
            ],
        },
        FolderSpec {
            id: "folder_admin_gateway".to_string(),
            name: "🔐 Admin Gateway & Settings".to_string(),
            tag: "Admin & Settings".to_string(),
            description: "Superuser authentication, admin accounts, and system settings.".to_string(),
            endpoints: vec![
                EndpointSpec {
                    id: "adm_has_admins".to_string(),
                    name: "Check Has Admins".to_string(),
                    method: "GET".to_string(),
                    path_template: "/api/admins/has-admins".to_string(),
                    path_params: vec![],
                    query: vec![],
                    headers: vec![],
                    body: BodySpec::None,
                    auth_required: false,
                    description: "Check if at least one Superuser exists in the system.".to_string(),
                    test_script: None,
                },
                EndpointSpec {
                    id: "adm_login".to_string(),
                    name: "Admin Auth with Password (Login)".to_string(),
                    method: "POST".to_string(),
                    path_template: "/api/admins/auth-with-password".to_string(),
                    path_params: vec![],
                    query: vec![],
                    headers: vec![],
                    body: BodySpec::Json(json!({
                        "identity": "{{admin_email}}",
                        "password": "{{admin_password}}"
                    })),
                    auth_required: false,
                    description: "Authenticate Superuser with email and password. Automatically sets the 'auth_token' collection variable.".to_string(),
                    test_script: Some(admin_login_test),
                },
                EndpointSpec {
                    id: "adm_list".to_string(),
                    name: "List Admins".to_string(),
                    method: "GET".to_string(),
                    path_template: "/api/admins".to_string(),
                    path_params: vec![],
                    query: vec![],
                    headers: vec![],
                    body: BodySpec::None,
                    auth_required: true,
                    description: "List all superuser accounts (Requires Bearer Token).".to_string(),
                    test_script: None,
                },
                EndpointSpec {
                    id: "adm_create".to_string(),
                    name: "Create Superuser Admin".to_string(),
                    method: "POST".to_string(),
                    path_template: "/api/admins".to_string(),
                    path_params: vec![],
                    query: vec![],
                    headers: vec![],
                    body: BodySpec::Json(json!({
                        "email": "new_admin@example.com",
                        "password": "Password_Admin_123!"
                    })),
                    auth_required: true,
                    description: "Create a new Superuser (Requires Bearer Token or first initialization).".to_string(),
                    test_script: None,
                },
                EndpointSpec {
                    id: "adm_get_settings".to_string(),
                    name: "Get System Settings".to_string(),
                    method: "GET".to_string(),
                    path_template: "/api/settings".to_string(),
                    path_params: vec![],
                    query: vec![],
                    headers: vec![],
                    body: BodySpec::None,
                    auth_required: true,
                    description: "Get PadiBase system configuration & metadata (Requires Bearer Token).".to_string(),
                    test_script: None,
                },
                EndpointSpec {
                    id: "adm_update_settings".to_string(),
                    name: "Update System Settings".to_string(),
                    method: "PATCH".to_string(),
                    path_template: "/api/settings".to_string(),
                    path_params: vec![],
                    query: vec![],
                    headers: vec![],
                    body: BodySpec::Json(json!({
                        "meta": {
                            "appName": "PadiBase Custom App",
                            "appUrl": "http://127.0.0.1:8085"
                        }
                    })),
                    auth_required: true,
                    description: "Update system settings (Requires Bearer Token).".to_string(),
                    test_script: None,
                },
            ],
        },
        FolderSpec {
            id: "folder_tables_schema".to_string(),
            name: "📦 Tables Schema Management".to_string(),
            tag: "Tables Schema".to_string(),
            description: "Inspect, create, update, and synchronize database table schemas.".to_string(),
            endpoints: vec![
                EndpointSpec {
                    id: "tbl_list".to_string(),
                    name: "List All Tables".to_string(),
                    method: "GET".to_string(),
                    path_template: "/api/tables".to_string(),
                    path_params: vec![],
                    query: vec![],
                    headers: vec![],
                    body: BodySpec::None,
                    auth_required: true,
                    description: "List all tables and their schema definitions.".to_string(),
                    test_script: None,
                },
                EndpointSpec {
                    id: "tbl_sync".to_string(),
                    name: "Sync Tables With Database".to_string(),
                    method: "POST".to_string(),
                    path_template: "/api/tables/sync".to_string(),
                    path_params: vec![],
                    query: vec![],
                    headers: vec![],
                    body: BodySpec::None,
                    auth_required: true,
                    description: "Introspect database tables and automatically synchronize schema definitions.".to_string(),
                    test_script: None,
                },
                EndpointSpec {
                    id: "tbl_create".to_string(),
                    name: "Create Table".to_string(),
                    method: "POST".to_string(),
                    path_template: "/api/tables".to_string(),
                    path_params: vec![],
                    query: vec![],
                    headers: vec![],
                    body: BodySpec::Json(json!({
                        "name": "posts",
                        "type": "base",
                        "realtime": true,
                        "schema": [
                            { "name": "title", "type": "text", "required": true },
                            { "name": "content", "type": "text", "required": false },
                            { "name": "published", "type": "bool", "required": false }
                        ]
                    })),
                    auth_required: true,
                    description: "Create a new Base or Auth table and automatically generate SQL table.".to_string(),
                    test_script: None,
                },
                EndpointSpec {
                    id: "tbl_get".to_string(),
                    name: format!("Get Table Schema ({})", sample_table),
                    method: "GET".to_string(),
                    path_template: format!("/api/tables/{}", sample_table),
                    path_params: vec![],
                    query: vec![],
                    headers: vec![],
                    body: BodySpec::None,
                    auth_required: true,
                    description: format!("Get detailed schema and rule definitions for table '{}'.", sample_table),
                    test_script: None,
                },
            ],
        },
    ]
}

fn build_custom_endpoints_folder(custom_endpoints: &[CustomEndpoint]) -> FolderSpec {
    let mut endpoints = Vec::new();
    let mut has_transfer = false;

    for (idx, ep) in custom_endpoints.iter().enumerate() {
        let resolved = resolve_custom_endpoint_path(&ep.path);
        if resolved.to_lowercase().ends_with("/transfer") {
            has_transfer = true;
        }
        let (path_param_names, sql_params) = extract_custom_endpoint_params(&resolved, &ep.sql);
        let path_template = normalize_path_template(&resolved);

        let mut method = ep.method.trim().to_uppercase();
        if method.is_empty() || method == "ANY" || method == "*" {
            let trimmed_sql = ep.sql.trim().to_uppercase();
            if trimmed_sql.starts_with("INSERT")
                || trimmed_sql.starts_with("UPDATE")
                || trimmed_sql.starts_with("DELETE")
                || trimmed_sql.starts_with("REPLACE")
            {
                method = "POST".to_string();
            } else {
                method = "GET".to_string();
            }
        }

        let path_params: Vec<PathParamSpec> = path_param_names
            .iter()
            .map(|p| PathParamSpec {
                name: p.clone(),
                example: match sample_value_for_param_name(p) {
                    Value::String(s) => s,
                    other => other.to_string(),
                },
                description: format!("Path parameter '{}'", p),
            })
            .collect();

        let mut query = Vec::new();
        let body = if matches!(method.as_str(), "GET" | "DELETE" | "HEAD") {
            for p in &sql_params {
                let sample = match sample_value_for_param_name(p) {
                    Value::String(s) => s,
                    other => other.to_string(),
                };
                query.push(QueryParamSpec {
                    key: p.clone(),
                    value: sample,
                    disabled: false,
                    description: format!("SQL parameter :{}", p),
                });
            }
            BodySpec::None
        } else {
            let mut body_map = Map::new();
            for p in &sql_params {
                body_map.insert(p.clone(), sample_value_for_param_name(p));
            }
            BodySpec::Json(Value::Object(body_map))
        };

        let auth_rule = ep.auth_rule.trim();
        let is_public = auth_rule.is_empty()
            || auth_rule.eq_ignore_ascii_case("public")
            || auth_rule == "*"
            || auth_rule.eq_ignore_ascii_case("anyone")
            || auth_rule.eq_ignore_ascii_case("allow_all");

        let mut desc_parts = Vec::new();
        if !ep.description.trim().is_empty() {
            desc_parts.push(ep.description.trim().to_string());
        }
        desc_parts.push(format!(
            "Access Rule: {}",
            if auth_rule.is_empty() { "public" } else { auth_rule }
        ));
        if !ep.sql.trim().is_empty() {
            let compact: String = ep
                .sql
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ")
                .chars()
                .take(180)
                .collect();
            desc_parts.push(format!("SQL: {}", compact));
        }

        let ep_id = if ep.id.is_empty() {
            format!("custom_ep_{}", idx)
        } else {
            ep.id.clone()
        };

        endpoints.push(EndpointSpec {
            id: ep_id,
            name: if ep.name.trim().is_empty() {
                resolved.clone()
            } else {
                ep.name.clone()
            },
            method,
            path_template,
            path_params,
            query,
            headers: vec![],
            body,
            auth_required: !is_public,
            description: desc_parts.join(" | "),
            test_script: None,
        });
    }

    if !has_transfer {
        endpoints.push(EndpointSpec {
            id: "custom_builtin_transfer".to_string(),
            name: "Custom Atomic Transfer (Built-in)".to_string(),
            method: "POST".to_string(),
            path_template: "/api/custom/transfer".to_string(),
            path_params: vec![],
            query: vec![],
            headers: vec![],
            body: BodySpec::Json(json!({
                "collection": "wallets",
                "from_id": "record_1",
                "to_id": "record_2",
                "field": "balance",
                "amount": 50
            })),
            auth_required: true,
            description: "Built-in custom transactional endpoint that atomically transfers balance/stock between two records.".to_string(),
            test_script: None,
        });
    }

    FolderSpec {
        id: "folder_custom_apis".to_string(),
        name: "✨ Custom API Endpoints".to_string(),
        tag: "Custom APIs".to_string(),
        description: "Dynamic SQL-powered custom REST endpoints configured in PadiBase Studio.".to_string(),
        endpoints,
    }
}

fn build_storage_folder() -> FolderSpec {
    FolderSpec {
        id: "folder_storage".to_string(),
        name: "🗂️ Storage & Files".to_string(),
        tag: "Storage & Files".to_string(),
        description: "Storage buckets management, multipart file uploads, and asset downloads.".to_string(),
        endpoints: vec![
            EndpointSpec {
                id: "stg_list".to_string(),
                name: "List Storage Buckets & Files".to_string(),
                method: "GET".to_string(),
                path_template: "/api/storage".to_string(),
                path_params: vec![],
                query: vec![],
                headers: vec![],
                body: BodySpec::None,
                auth_required: true,
                description: "List all storage buckets and uploaded files.".to_string(),
                test_script: None,
            },
            EndpointSpec {
                id: "stg_create_bucket".to_string(),
                name: "Create Storage Bucket".to_string(),
                method: "POST".to_string(),
                path_template: "/api/storage/buckets".to_string(),
                path_params: vec![],
                query: vec![],
                headers: vec![],
                body: BodySpec::Json(json!({ "name": "assets" })),
                auth_required: true,
                description: "Create a new storage bucket directory.".to_string(),
                test_script: None,
            },
            EndpointSpec {
                id: "stg_upload_avatar".to_string(),
                name: "Upload File to Bucket (avatar)".to_string(),
                method: "POST".to_string(),
                path_template: "/api/storage/avatar".to_string(),
                path_params: vec![],
                query: vec![],
                headers: vec![],
                body: BodySpec::FormData(vec![FormFieldSpec {
                    key: "file".to_string(),
                    is_file: true,
                    description: "File to upload into bucket 'avatar'".to_string(),
                }]),
                auth_required: true,
                description: "Upload a file via multipart/form-data to storage bucket 'avatar'.".to_string(),
                test_script: None,
            },
            EndpointSpec {
                id: "stg_get_avatar".to_string(),
                name: "Download File from Bucket (avatar)".to_string(),
                method: "GET".to_string(),
                path_template: "/api/storage/avatar/{filename}".to_string(),
                path_params: vec![PathParamSpec {
                    name: "filename".to_string(),
                    example: "sample.png".to_string(),
                    description: "Stored filename in bucket 'avatar'".to_string(),
                }],
                query: vec![],
                headers: vec![],
                body: BodySpec::None,
                auth_required: false,
                description: "Download or view a file stored in bucket 'avatar'.".to_string(),
                test_script: None,
            },
        ],
    }
}

fn build_ops_folder(custom_endpoints: &[CustomEndpoint]) -> FolderSpec {
    let sample_ep_id = custom_endpoints
        .first()
        .map(|e| e.id.as_str())
        .filter(|s| !s.is_empty())
        .unwrap_or("ENDPOINT_ID");

    FolderSpec {
        id: "folder_ops_sql_backups".to_string(),
        name: "🔄 Database Transactions, Custom SQL & Backups".to_string(),
        tag: "Transactions & Backups".to_string(),
        description: "Atomic multi-operation transactions, raw SQL execution, custom endpoint management, and backups.".to_string(),
        endpoints: vec![
            EndpointSpec {
                id: "ops_tx".to_string(),
                name: "Execute Atomic Batch Transaction".to_string(),
                method: "POST".to_string(),
                path_template: "/api/transactions".to_string(),
                path_params: vec![],
                query: vec![],
                headers: vec![],
                body: BodySpec::Json(json!({
                    "operations": [
                        { "action": "create", "collection": "posts", "data": { "title": "Atomic Post 1" } },
                        { "action": "create", "collection": "posts", "data": { "title": "Atomic Post 2" } }
                    ]
                })),
                auth_required: true,
                description: "Execute multiple create, update, delete, or admin SQL operations inside a single atomic database transaction.".to_string(),
                test_script: None,
            },
            EndpointSpec {
                id: "ops_raw_sql".to_string(),
                name: "Execute Custom Raw SQL Query (Admin)".to_string(),
                method: "POST".to_string(),
                path_template: "/api/sql".to_string(),
                path_params: vec![],
                query: vec![],
                headers: vec![],
                body: BodySpec::Json(json!({
                    "sql": "SELECT id, email, role FROM `users` WHERE `role` = ? LIMIT 10",
                    "params": ["user"],
                    "transaction": false
                })),
                auth_required: true,
                description: "Run custom parameterized SQL queries directly as Superuser/Admin.".to_string(),
                test_script: None,
            },
            EndpointSpec {
                id: "ops_ep_list".to_string(),
                name: "List Custom API Endpoints (Admin)".to_string(),
                method: "GET".to_string(),
                path_template: "/api/endpoints".to_string(),
                path_params: vec![],
                query: vec![],
                headers: vec![],
                body: BodySpec::None,
                auth_required: true,
                description: "List all saved Custom SQL API definitions.".to_string(),
                test_script: None,
            },
            EndpointSpec {
                id: "ops_ep_create".to_string(),
                name: "Save Custom API Endpoint (Admin)".to_string(),
                method: "POST".to_string(),
                path_template: "/api/endpoints".to_string(),
                path_params: vec![],
                query: vec![],
                headers: vec![],
                body: BodySpec::Json(json!({
                    "name": "Custom Summary",
                    "method": "GET",
                    "path": "api/custom/summary",
                    "sql": "SELECT COUNT(*) AS total FROM users",
                    "authRule": "public",
                    "description": "Sample custom SQL endpoint",
                    "transaction": false
                })),
                auth_required: true,
                description: "Create or update a Custom SQL API definition.".to_string(),
                test_script: None,
            },
            EndpointSpec {
                id: "ops_ep_delete".to_string(),
                name: "Delete Custom API Endpoint (Admin)".to_string(),
                method: "DELETE".to_string(),
                path_template: format!("/api/endpoints/{}", sample_ep_id),
                path_params: vec![],
                query: vec![],
                headers: vec![],
                body: BodySpec::None,
                auth_required: true,
                description: "Delete a saved Custom SQL API definition by ID.".to_string(),
                test_script: None,
            },
            EndpointSpec {
                id: "ops_backups_list".to_string(),
                name: "List Backups".to_string(),
                method: "GET".to_string(),
                path_template: "/api/backups".to_string(),
                path_params: vec![],
                query: vec![],
                headers: vec![],
                body: BodySpec::None,
                auth_required: true,
                description: "List available online backup snapshot zip files.".to_string(),
                test_script: None,
            },
            EndpointSpec {
                id: "ops_backups_create".to_string(),
                name: "Create Online Backup Snapshot".to_string(),
                method: "POST".to_string(),
                path_template: "/api/backups".to_string(),
                path_params: vec![],
                query: vec![],
                headers: vec![],
                body: BodySpec::None,
                auth_required: true,
                description: "Create an atomic online backup snapshot of database and files.".to_string(),
                test_script: None,
            },
        ],
    }
}

fn build_all_canonical_spec(
    cols: &[Collection],
    custom_endpoints: &[CustomEndpoint],
    base_url: &str,
) -> CanonicalSpec {
    let mut vars: BTreeMap<String, String> = BTreeMap::new();
    vars.insert("base_url".to_string(), base_url.to_string());
    vars.insert("auth_token".to_string(), String::new());
    vars.insert("admin_token".to_string(), String::new());
    vars.insert("admin_email".to_string(), "admin@example.com".to_string());
    vars.insert("admin_password".to_string(), "password_rahasia_123".to_string());
    vars.insert("record_id".to_string(), "1".to_string());
    vars.insert("filename".to_string(), "sample.png".to_string());

    for col in cols {
        let id_var = format!("{}_id", singular_name(&col.name));
        vars.entry(id_var).or_insert_with(|| "1".to_string());
    }

    let mut folders = build_system_and_admin_folders(cols);
    let custom_folder = build_custom_endpoints_folder(custom_endpoints);
    for ep in &custom_folder.endpoints {
        for pp in &ep.path_params {
            vars.entry(pp.name.clone())
                .or_insert_with(|| pp.example.clone());
        }
    }
    folders.push(custom_folder);
    folders.push(build_storage_folder());
    folders.push(build_ops_folder(custom_endpoints));

    for col in cols {
        folders.push(FolderSpec {
            id: format!("folder_table_{}", col.name),
            name: format!("📂 {}", pascal_case(&col.name)),
            tag: pascal_case(&col.name),
            description: format!(
                "CRUD, Padi Rust endpoints, Realtime, Import, and Export for collection '{}'",
                col.name
            ),
            endpoints: build_table_endpoints(col),
        });
    }

    CanonicalSpec {
        title: "PadiBase API Collection (Full)".to_string(),
        description: "Complete REST Client API Collection for PadiBase Backend-as-a-Service, matching Padi Rust standards. Compatible with Postman, Insomnia, Thunder Client, Bruno, and VS Code REST Client.".to_string(),
        base_url: base_url.to_string(),
        is_single: false,
        variables: vars.into_iter().collect(),
        folders,
    }
}

fn build_single_canonical_spec(col: &Collection, base_url: &str) -> CanonicalSpec {
    let singular = singular_name(&col.name);
    let title = pascal_case(&col.name);
    let id_var_key = format!("{}_id", singular);

    let variables = vec![
        ("base_url".to_string(), base_url.to_string()),
        ("auth_token".to_string(), String::new()),
        ("record_id".to_string(), "1".to_string()),
        ("filename".to_string(), "sample.png".to_string()),
        (id_var_key, "1".to_string()),
    ];

    CanonicalSpec {
        title: format!("{} API Collection (PadiBase)", title),
        description: format!(
            "REST Client API Collection for '{}' generated by PadiBase (compatible with Postman, Insomnia, Thunder Client, Bruno, and VS Code REST Client).",
            col.name
        ),
        base_url: base_url.to_string(),
        is_single: true,
        variables,
        folders: vec![FolderSpec {
            id: format!("folder_table_{}", col.name),
            name: format!("📂 {}", title),
            tag: title,
            description: format!("Endpoints for collection '{}'", col.name),
            endpoints: build_table_endpoints(col),
        }],
    }
}

fn endpoint_to_postman_item(ep: &EndpointSpec) -> Value {
    let mut headers: Vec<Value> = ep
        .headers
        .iter()
        .map(|(k, v)| json!({ "key": k, "value": v }))
        .collect();

    if matches!(ep.body, BodySpec::Json(_))
        && !ep
            .headers
            .iter()
            .any(|(k, _)| k.eq_ignore_ascii_case("content-type"))
    {
        headers.push(json!({ "key": "Content-Type", "value": "application/json" }));
    }

    let mut req = Map::new();
    req.insert("method".to_string(), json!(ep.method));
    req.insert("header".to_string(), Value::Array(headers));
    req.insert(
        "url".to_string(),
        build_postman_url(&ep.path_template, &ep.query),
    );
    req.insert("description".to_string(), json!(ep.description));

    if !ep.auth_required {
        req.insert("auth".to_string(), json!({ "type": "noauth" }));
    }

    match &ep.body {
        BodySpec::None => {}
        BodySpec::Json(val) => {
            let raw = serde_json::to_string_pretty(val).unwrap_or_default();
            req.insert(
                "body".to_string(),
                json!({
                    "mode": "raw",
                    "raw": raw,
                    "options": {
                        "raw": { "language": "json" }
                    }
                }),
            );
        }
        BodySpec::FormData(fields) => {
            let formdata: Vec<Value> = fields
                .iter()
                .map(|f| {
                    json!({
                        "key": f.key,
                        "type": if f.is_file { "file" } else { "text" },
                        "description": f.description
                    })
                })
                .collect();
            req.insert(
                "body".to_string(),
                json!({
                    "mode": "formdata",
                    "formdata": formdata
                }),
            );
        }
    }

    let mut item = Map::new();
    item.insert("name".to_string(), json!(ep.name));
    item.insert("request".to_string(), Value::Object(req));

    if let Some(script) = &ep.test_script {
        let lines: Vec<String> = script.lines().map(String::from).collect();
        item.insert(
            "event".to_string(),
            json!([
                {
                    "listen": "test",
                    "script": {
                        "exec": lines,
                        "type": "text/javascript"
                    }
                }
            ]),
        );
    }

    Value::Object(item)
}

fn canonical_to_postman(spec: &CanonicalSpec) -> Value {
    let items: Vec<Value> = if spec.is_single {
        spec.folders
            .first()
            .map(|f| f.endpoints.iter().map(endpoint_to_postman_item).collect())
            .unwrap_or_default()
    } else {
        spec.folders
            .iter()
            .map(|f| {
                let folder_items: Vec<Value> =
                    f.endpoints.iter().map(endpoint_to_postman_item).collect();
                json!({
                    "name": f.name,
                    "description": f.description,
                    "item": folder_items
                })
            })
            .collect()
    };

    let vars: Vec<Value> = spec
        .variables
        .iter()
        .map(|(k, v)| json!({ "key": k, "value": v, "type": "string" }))
        .collect();

    json!({
        "info": {
            "_postman_id": Uuid::new_v4().to_string(),
            "name": spec.title,
            "description": spec.description,
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
        "variable": vars,
        "item": items
    })
}

fn canonical_to_insomnia(spec: &CanonicalSpec) -> Value {
    let now_ms = chrono::Utc::now().timestamp_millis();
    let ws_id = "wrk_padibase_studio";
    let env_id = "env_padibase_base";

    let mut env_data = Map::new();
    for (k, v) in &spec.variables {
        env_data.insert(k.clone(), Value::String(v.clone()));
    }

    let mut resources = vec![
        json!({
            "_id": ws_id,
            "parentId": Value::Null,
            "modified": now_ms,
            "created": now_ms,
            "name": spec.title,
            "description": spec.description,
            "scope": "collection",
            "_type": "workspace"
        }),
        json!({
            "_id": env_id,
            "parentId": ws_id,
            "modified": now_ms,
            "created": now_ms,
            "name": "Base Environment",
            "data": Value::Object(env_data),
            "isPrivate": false,
            "_type": "environment"
        }),
    ];

    for (f_idx, folder) in spec.folders.iter().enumerate() {
        let group_id = format!("fld_{}_{}", folder.id, f_idx);
        resources.push(json!({
            "_id": group_id,
            "parentId": ws_id,
            "modified": now_ms,
            "created": now_ms,
            "name": folder.name,
            "description": folder.description,
            "metaSortKey": (f_idx as i64) * 100,
            "_type": "request_group"
        }));

        for (e_idx, ep) in folder.endpoints.iter().enumerate() {
            let mut headers: Vec<Value> = ep
                .headers
                .iter()
                .map(|(k, v)| json!({ "name": k, "value": v }))
                .collect();

            let body = match &ep.body {
                BodySpec::None => json!({}),
                BodySpec::Json(val) => {
                    if !ep
                        .headers
                        .iter()
                        .any(|(k, _)| k.eq_ignore_ascii_case("content-type"))
                    {
                        headers.push(json!({ "name": "Content-Type", "value": "application/json" }));
                    }
                    json!({
                        "mimeType": "application/json",
                        "text": serde_json::to_string_pretty(val).unwrap_or_default()
                    })
                }
                BodySpec::FormData(fields) => {
                    let params: Vec<Value> = fields
                        .iter()
                        .map(|f| {
                            json!({
                                "name": f.key,
                                "type": if f.is_file { "file" } else { "text" },
                                "value": "",
                                "description": f.description
                            })
                        })
                        .collect();
                    json!({
                        "mimeType": "multipart/form-data",
                        "params": params
                    })
                }
            };

            let parameters: Vec<Value> = ep
                .query
                .iter()
                .map(|q| {
                    json!({
                        "name": q.key,
                        "value": q.value,
                        "disabled": q.disabled,
                        "description": q.description
                    })
                })
                .collect();

            let auth = if ep.auth_required {
                json!({ "type": "bearer", "token": "{{ _.auth_token }}" })
            } else {
                json!({})
            };

            resources.push(json!({
                "_id": format!("req_{}_{}_{}", ep.id, f_idx, e_idx),
                "parentId": group_id,
                "modified": now_ms,
                "created": now_ms,
                "url": format!("{{{{ _.base_url }}}}{}", replace_path_vars_insomnia(&ep.path_template)),
                "name": ep.name,
                "description": ep.description,
                "method": ep.method,
                "body": body,
                "parameters": parameters,
                "headers": headers,
                "authentication": auth,
                "metaSortKey": (e_idx as i64) * 10,
                "_type": "request"
            }));
        }
    }

    json!({
        "_type": "export",
        "__export_format": 4,
        "__export_date": chrono::Utc::now().to_rfc3339(),
        "__export_source": "padibase.studio:v1",
        "resources": resources
    })
}

fn infer_openapi_schema(val: &Value) -> Value {
    match val {
        Value::Null => json!({ "type": "string" }),
        Value::Bool(_) => json!({ "type": "boolean" }),
        Value::Number(n) => {
            if n.is_i64() || n.is_u64() {
                json!({ "type": "integer" })
            } else {
                json!({ "type": "number" })
            }
        }
        Value::String(_) => json!({ "type": "string" }),
        Value::Array(arr) => {
            let items = arr
                .first()
                .map(infer_openapi_schema)
                .unwrap_or_else(|| json!({ "type": "object" }));
            json!({ "type": "array", "items": items })
        }
        Value::Object(map) => {
            let mut props = Map::new();
            for (k, v) in map {
                props.insert(k.clone(), infer_openapi_schema(v));
            }
            json!({ "type": "object", "properties": props })
        }
    }
}

fn canonical_to_openapi(spec: &CanonicalSpec) -> Value {
    let mut tags = Vec::new();
    let mut tag_set = HashSet::new();
    let mut paths: Map<String, Value> = Map::new();

    for folder in &spec.folders {
        if tag_set.insert(folder.tag.clone()) {
            tags.push(json!({
                "name": folder.tag,
                "description": folder.description
            }));
        }

        for ep in &folder.endpoints {
            let p_entry = paths
                .entry(ep.path_template.clone())
                .or_insert_with(|| Value::Object(Map::new()));
            let Some(method_map) = p_entry.as_object_mut() else {
                continue;
            };
            let method_key = ep.method.to_lowercase();

            let mut params = Vec::new();
            for pp in &ep.path_params {
                params.push(json!({
                    "name": pp.name,
                    "in": "path",
                    "required": true,
                    "schema": { "type": "string" },
                    "example": pp.example,
                    "description": pp.description
                }));
            }
            for q in &ep.query {
                let mut q_param = Map::new();
                q_param.insert("name".to_string(), json!(q.key));
                q_param.insert("in".to_string(), json!("query"));
                q_param.insert("required".to_string(), json!(false));
                q_param.insert("schema".to_string(), json!({ "type": "string" }));
                if !q.value.is_empty() {
                    q_param.insert("example".to_string(), json!(q.value));
                }
                if !q.description.is_empty() {
                    q_param.insert("description".to_string(), json!(q.description));
                }
                params.push(Value::Object(q_param));
            }

            let mut op = Map::new();
            op.insert("tags".to_string(), json!([folder.tag]));
            op.insert("summary".to_string(), json!(ep.name));
            op.insert("description".to_string(), json!(ep.description));
            op.insert("operationId".to_string(), json!(ep.id));
            if !params.is_empty() {
                op.insert("parameters".to_string(), Value::Array(params));
            }
            if ep.auth_required {
                op.insert("security".to_string(), json!([{ "bearerAuth": [] }]));
            } else {
                op.insert("security".to_string(), json!([]));
            }

            match &ep.body {
                BodySpec::None => {}
                BodySpec::Json(val) => {
                    op.insert(
                        "requestBody".to_string(),
                        json!({
                            "required": true,
                            "content": {
                                "application/json": {
                                    "schema": infer_openapi_schema(val),
                                    "example": val
                                }
                            }
                        }),
                    );
                }
                BodySpec::FormData(fields) => {
                    let mut props = Map::new();
                    for f in fields {
                        if f.is_file {
                            props.insert(
                                f.key.clone(),
                                json!({ "type": "string", "format": "binary", "description": f.description }),
                            );
                        } else {
                            props.insert(
                                f.key.clone(),
                                json!({ "type": "string", "description": f.description }),
                            );
                        }
                    }
                    op.insert(
                        "requestBody".to_string(),
                        json!({
                            "required": true,
                            "content": {
                                "multipart/form-data": {
                                    "schema": {
                                        "type": "object",
                                        "properties": props
                                    }
                                }
                            }
                        }),
                    );
                }
            }

            op.insert(
                "responses".to_string(),
                json!({
                    "200": { "description": "Successful response" }
                }),
            );

            if !method_map.contains_key(&method_key) || folder.tag == "Custom APIs" {
                method_map.insert(method_key, Value::Object(op));
            }
        }
    }

    json!({
        "openapi": "3.0.3",
        "info": {
            "title": spec.title,
            "version": "1.0.0",
            "description": spec.description
        },
        "servers": [
            {
                "url": spec.base_url,
                "description": "PadiBase Server"
            }
        ],
        "tags": tags,
        "components": {
            "securitySchemes": {
                "bearerAuth": {
                    "type": "http",
                    "scheme": "bearer",
                    "bearerFormat": "JWT"
                }
            }
        },
        "paths": Value::Object(paths)
    })
}

/// Build list of Postman REST request items for a single collection
pub fn build_collection_items(col: &Collection) -> Vec<Value> {
    build_table_endpoints(col)
        .iter()
        .map(endpoint_to_postman_item)
        .collect()
}

/// Build Postman Collection v2.1.0 for a single collection
pub fn build_single_collection_json(col: &Collection, base_url: &str) -> Value {
    let spec = build_single_canonical_spec(col, base_url);
    canonical_to_postman(&spec)
}

/// Build complete Postman Collection v2.1.0 for all collections, custom endpoints, storage, and core system endpoints
pub fn build_all_collections_with_custom_json(
    cols: &[Collection],
    custom_endpoints: &[CustomEndpoint],
    base_url: &str,
) -> Value {
    let spec = build_all_canonical_spec(cols, custom_endpoints, base_url);
    canonical_to_postman(&spec)
}

/// Build complete Postman Collection v2.1.0 for all collections and core system endpoints
pub fn build_all_collections_json(cols: &[Collection], base_url: &str) -> Value {
    build_all_collections_with_custom_json(cols, &[], base_url)
}

/// Build Insomnia Export v4 JSON for all collections, custom endpoints, storage, and system endpoints
pub fn build_insomnia_v4_json(
    cols: &[Collection],
    custom_endpoints: &[CustomEndpoint],
    base_url: &str,
) -> Value {
    let spec = build_all_canonical_spec(cols, custom_endpoints, base_url);
    canonical_to_insomnia(&spec)
}

/// Build Insomnia Export v4 JSON for a single collection
pub fn build_single_insomnia_v4_json(col: &Collection, base_url: &str) -> Value {
    let spec = build_single_canonical_spec(col, base_url);
    canonical_to_insomnia(&spec)
}

/// Build OpenAPI 3.0.3 JSON for all collections, custom endpoints, storage, and system endpoints
pub fn build_openapi_v3_json(
    cols: &[Collection],
    custom_endpoints: &[CustomEndpoint],
    base_url: &str,
) -> Value {
    let spec = build_all_canonical_spec(cols, custom_endpoints, base_url);
    canonical_to_openapi(&spec)
}

/// Build OpenAPI 3.0.3 JSON for a single collection
pub fn build_single_openapi_v3_json(col: &Collection, base_url: &str) -> Value {
    let spec = build_single_canonical_spec(col, base_url);
    canonical_to_openapi(&spec)
}
