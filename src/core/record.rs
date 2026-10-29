use anyhow::{anyhow, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use super::db::{json_get_i64, json_get_str, DbPool, Val};
use super::schema::{generate_id, Collection, CollectionType, FieldType};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Record {
    pub id: String,
    pub created: String,
    pub updated: String,
    #[serde(rename = "collectionId")]
    pub collection_id: String,
    #[serde(rename = "collectionName")]
    pub collection_name: String,
    #[serde(flatten)]
    pub data: Map<String, Value>,
}

pub struct PaginatedRecords {
    pub page: i64,
    pub per_page: i64,
    pub total_items: i64,
    pub total_pages: i64,
    pub items: Vec<Record>,
}

impl Serialize for PaginatedRecords {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("PaginatedRecords", 5)?;
        state.serialize_field("page", &self.page)?;
        state.serialize_field("perPage", &self.per_page)?;
        state.serialize_field("totalItems", &self.total_items)?;
        state.serialize_field("totalPages", &self.total_pages)?;
        state.serialize_field("items", &self.items)?;
        state.end()
    }
}

pub async fn create_record(
    pool: &DbPool,
    col: &Collection,
    mut payload: Map<String, Value>,
) -> Result<Record> {
    let now = Utc::now().to_rfc3339();
    let id = payload
        .get("id")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(generate_id);

    payload.insert("id".to_string(), Value::String(id.clone()));
    payload.insert("created".to_string(), Value::String(now.clone()));
    payload.insert("updated".to_string(), Value::String(now.clone()));

    if col.collection_type == CollectionType::Auth {
        if let Some(pass_val) = payload.remove("password") {
            if let Some(pass_str) = pass_val.as_str() {
                let hash = crate::core::auth::hash_password(pass_str)?;
                payload.insert("passwordHash".to_string(), Value::String(hash.clone()));
                payload.insert("password".to_string(), Value::String(hash));
            }
        }
        if !payload.contains_key("tokenKey") {
            payload.insert(
                "tokenKey".to_string(),
                Value::String(crate::core::auth::generate_token_key()),
            );
        }

        if !payload.contains_key("role") {
            payload.insert("role".to_string(), Value::String("user".to_string()));
        }
        if !payload.contains_key("status") {
            payload.insert("status".to_string(), Value::String("active".to_string()));
        }

        // Keep created_at & updated_at in sync with created & updated (Padi standard)
        if !payload.contains_key("created_at") {
            payload.insert("created_at".to_string(), Value::String(now.clone()));
        }
        if !payload.contains_key("updated_at") {
            payload.insert("updated_at".to_string(), Value::String(now.clone()));
        }

        // Auto-generate username if not provided
        let has_username = payload
            .get("username")
            .and_then(|u| u.as_str())
            .map(|s| !s.trim().is_empty())
            .unwrap_or(false);
        if !has_username {
            let default_username =
                if let Some(email) = payload.get("email").and_then(|e| e.as_str()) {
                    email.split('@').next().unwrap_or("user").to_string()
                } else {
                    format!("user_{}", &id[..6.min(id.len())])
                };
            payload.insert("username".to_string(), Value::String(default_username));
        }
    }

    let mut keys = Vec::new();
    let mut placeholders = Vec::new();
    let mut params = Vec::new();

    for (k, v) in &payload {
        keys.push(format!("`{}`", k));
        placeholders.push("?");
        params.push(Val::from(v));
    }

    let sql = format!(
        "INSERT INTO `{}` ({}) VALUES ({});",
        col.name,
        keys.join(", "),
        placeholders.join(", ")
    );

    pool.execute(&sql, &params).await?;

    get_record(pool, col, &id)
        .await?
        .ok_or_else(|| anyhow!("Record was not created"))
}

pub async fn get_record(pool: &DbPool, col: &Collection, id: &str) -> Result<Option<Record>> {
    let sql = format!("SELECT * FROM `{}` WHERE `id` = ? LIMIT 1;", col.name);
    let row_opt = pool.fetch_optional(&sql, &[Val::from(id)]).await?;

    if let Some(row) = row_opt {
        Ok(Some(json_row_to_record(&row, col)?))
    } else {
        Ok(None)
    }
}

pub async fn update_record(
    pool: &DbPool,
    col: &Collection,
    id: &str,
    mut payload: Map<String, Value>,
) -> Result<Record> {
    let now = Utc::now().to_rfc3339();
    payload.remove("id");
    payload.remove("created");
    payload.remove("collectionId");
    payload.remove("collectionName");
    payload.insert("updated".to_string(), Value::String(now.clone()));

    if col.collection_type == CollectionType::Auth {
        payload.insert("updated_at".to_string(), Value::String(now.clone()));
        if let Some(pass_val) = payload.remove("password") {
            if let Some(pass_str) = pass_val.as_str() {
                if !pass_str.trim().is_empty() {
                    let hash = crate::core::auth::hash_password(pass_str)?;
                    payload.insert("passwordHash".to_string(), Value::String(hash.clone()));
                    payload.insert("password".to_string(), Value::String(hash));
                }
            }
        }
    }

    let mut set_clauses = Vec::new();
    let mut params = Vec::new();

    for (k, v) in &payload {
        set_clauses.push(format!("`{}` = ?", k));
        params.push(Val::from(v));
    }
    params.push(Val::from(id));

    let sql = format!(
        "UPDATE `{}` SET {} WHERE `id` = ?;",
        col.name,
        set_clauses.join(", ")
    );

    let (rows_affected, _) = pool.execute(&sql, &params).await?;
    if rows_affected == 0 {
        return Err(anyhow!("Record not found: {}", id));
    }

    get_record(pool, col, id)
        .await?
        .ok_or_else(|| anyhow!("Record not found after update"))
}

pub async fn delete_record(pool: &DbPool, col: &Collection, id: &str) -> Result<bool> {
    let sql = format!("DELETE FROM `{}` WHERE `id` = ?;", col.name);
    let (rows_affected, _) = pool.execute(&sql, &[Val::from(id)]).await?;
    Ok(rows_affected > 0)
}

fn parse_pocketbase_filter(raw: &str, col: &Collection) -> String {
    let f = raw.trim();
    if f.is_empty() {
        return String::new();
    }

    let has_operator = f.contains('=')
        || f.contains('~')
        || f.contains('>')
        || f.contains('<')
        || f.to_uppercase().contains(" LIKE ")
        || f.to_uppercase().contains(" IS ");

    if !has_operator {
        // Quick keyword search across id and text columns
        let safe_kw = f.replace('\'', "''");
        let mut clauses = vec![format!("`id` LIKE '%{}%'", safe_kw)];
        if col.collection_type == CollectionType::Auth {
            clauses.push(format!("`email` LIKE '%{}%'", safe_kw));
            clauses.push(format!("`username` LIKE '%{}%'", safe_kw));
        }
        for field in &col.schema {
            if matches!(
                field.field_type,
                FieldType::Text | FieldType::Email | FieldType::Url | FieldType::Select
            ) {
                clauses.push(format!("`{}` LIKE '%{}%'", field.name, safe_kw));
            }
        }
        return format!("WHERE ({})", clauses.join(" OR "));
    }

    // Translate PocketBase filter syntax to SQL WHERE clause
    let mut expr = f.to_string();
    expr = expr.replace("&&", " AND ");
    expr = expr.replace("||", " OR ");
    expr = expr.replace("!~", " NOT LIKE ");
    expr = expr.replace('~', " LIKE ");
    expr = expr.replace("= true", "= 1").replace("=true", "=1");
    expr = expr.replace("= false", "= 0").replace("=false", "=0");
    format!("WHERE {}", expr)
}

pub async fn list_records(
    pool: &DbPool,
    col: &Collection,
    page: i64,
    per_page: i64,
    sort: Option<&str>,
    filter: Option<&str>,
) -> Result<PaginatedRecords> {
    let page = if page < 1 { 1 } else { page };
    let per_page = if per_page < 1 { 30 } else { per_page };
    let offset = (page - 1) * per_page;

    let filter_sql = filter
        .map(|f| parse_pocketbase_filter(f, col))
        .unwrap_or_default();

    let sort_sql = if let Some(s) = sort {
        if !s.trim().is_empty() {
            let parts: Vec<String> = s
                .split(',')
                .map(|item| {
                    let item = item.trim();
                    if let Some(stripped) = item.strip_prefix('-') {
                        format!("`{}` DESC", stripped)
                    } else if let Some(stripped) = item.strip_prefix('+') {
                        format!("`{}` ASC", stripped)
                    } else {
                        format!("`{}` ASC", item)
                    }
                })
                .collect();
            format!("ORDER BY {}", parts.join(", "))
        } else {
            "ORDER BY `created` DESC".to_string()
        }
    } else {
        "ORDER BY `created` DESC".to_string()
    };

    // Count query
    let count_sql = format!(
        "SELECT COUNT(*) as `total` FROM `{}` {};",
        col.name, filter_sql
    );
    let total_row = pool.fetch_one(&count_sql, &[]).await?;
    let total_items = json_get_i64(&total_row, "total");
    let total_pages = if total_items == 0 {
        1
    } else {
        (total_items as f64 / per_page as f64).ceil() as i64
    };

    // Items query
    let sql = format!(
        "SELECT * FROM `{}` {} {} LIMIT ? OFFSET ?;",
        col.name, filter_sql, sort_sql
    );
    let rows = pool
        .fetch_all(&sql, &[Val::Int(per_page), Val::Int(offset)])
        .await?;

    let mut items = Vec::new();
    for row in rows {
        items.push(json_row_to_record(&row, col)?);
    }

    Ok(PaginatedRecords {
        page,
        per_page,
        total_items,
        total_pages,
        items,
    })
}

fn canonical_field_name<'a>(raw_name: &'a str, col: &'a Collection) -> &'a str {
    if raw_name.eq_ignore_ascii_case("username") {
        return "username";
    }
    if raw_name.eq_ignore_ascii_case("email") {
        return "email";
    }
    if raw_name.eq_ignore_ascii_case("emailVisibility") {
        return "emailVisibility";
    }
    if raw_name.eq_ignore_ascii_case("verified") {
        return "verified";
    }
    for f in &col.schema {
        if f.name.eq_ignore_ascii_case(raw_name) {
            return &f.name;
        }
    }
    raw_name
}

fn json_row_to_record(row: &Value, col: &Collection) -> Result<Record> {
    let mut data = Map::new();
    let id = json_get_str(row, "id");
    let created = json_get_str(row, "created");
    let updated = json_get_str(row, "updated");

    if let Some(obj) = row.as_object() {
        for (raw_name, raw_val) in obj {
            if raw_name.eq_ignore_ascii_case("id")
                || raw_name.eq_ignore_ascii_case("created")
                || raw_name.eq_ignore_ascii_case("updated")
            {
                continue;
            }

            // Do not expose sensitive credentials in record output (matching padi_rust / padi_go / padi_php hidden fields)
            if raw_name.eq_ignore_ascii_case("passwordHash")
                || raw_name.eq_ignore_ascii_case("password")
                || raw_name.eq_ignore_ascii_case("tokenKey")
                || raw_name.eq_ignore_ascii_case("remember_token")
            {
                continue;
            }

            let name = canonical_field_name(raw_name, col);

            let is_bool_field = name == "verified"
                || name == "emailVisibility"
                || col
                    .schema
                    .iter()
                    .any(|f| f.name == name && f.field_type == FieldType::Bool);

            let is_json_field = col
                .schema
                .iter()
                .any(|f| f.name == name && f.field_type == FieldType::Json);

            let val: Value = if is_bool_field {
                match raw_val {
                    Value::Bool(b) => Value::Bool(*b),
                    Value::Number(n) => Value::Bool(n.as_i64().unwrap_or(0) != 0),
                    Value::String(s) => {
                        Value::Bool(s == "1" || s.eq_ignore_ascii_case("true"))
                    }
                    _ => Value::Bool(false),
                }
            } else if is_json_field {
                if let Value::String(s) = raw_val {
                    serde_json::from_str::<Value>(s).unwrap_or_else(|_| Value::String(s.clone()))
                } else {
                    raw_val.clone()
                }
            } else {
                raw_val.clone()
            };

            data.insert(name.to_string(), val);
        }
    }

    Ok(Record {
        id,
        created,
        updated,
        collection_id: col.id.clone(),
        collection_name: col.name.clone(),
        data,
    })
}
