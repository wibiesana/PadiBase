use anyhow::{anyhow, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use super::db::{json_get_i64, json_get_str, DbPool, DbTx, TableMetadata, Val};
use super::schema::{generate_id, Collection, CollectionType, FieldType};

#[derive(Debug, Clone, Deserialize)]
pub struct Record {
    pub id: String,
    pub data: Map<String, Value>,
}

impl Serialize for Record {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(None)?;

        // Pastikan id selalu ada di urutan teratas
        if !self.data.contains_key("id") {
            map.serialize_entry("id", &self.id)?;
        }

        for (k, v) in &self.data {
            map.serialize_entry(k, v)?;
        }
        map.end()
    }
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

fn pk_to_val(id: &str, meta: &TableMetadata) -> Val {
    if meta.pk_is_int {
        if let Ok(num) = id.parse::<i64>() {
            return Val::Int(num);
        }
    }
    Val::from(id)
}

fn resolve_timestamp_val(meta: &TableMetadata, col_name: &str, rfc_now: &str) -> Value {
    let col = meta.columns.iter().find(|c| c.name.eq_ignore_ascii_case(col_name));
    if let Some(c) = col {
        let dt = c.data_type.to_lowercase();
        if dt.contains("int") {
            return Value::from(Utc::now().timestamp());
        }
        if dt.contains("date") || dt.contains("time") {
            return Value::String(Utc::now().format("%Y-%m-%d %H:%M:%S").to_string());
        }
    }
    Value::String(rfc_now.to_string())
}

fn apply_schema_defaults(col: &Collection, payload: &mut Map<String, Value>) {
    for field in &col.schema {
        let missing_or_null = match payload.get(&field.name) {
            None | Some(Value::Null) => true,
            _ => false,
        };
        if missing_or_null {
            if let Some(default_val) = field.options.get("default") {
                if !default_val.is_null() {
                    payload.insert(field.name.clone(), default_val.clone());
                }
            }
        }
    }
}

pub async fn create_record(
    pool: &DbPool,
    col: &Collection,
    mut payload: Map<String, Value>,
) -> Result<Record> {
    let meta = super::db::get_table_metadata(pool, &col.name).await?;
    let now = Utc::now().to_rfc3339();

    apply_schema_defaults(col, &mut payload);

    // Primary key detection
    let custom_id = payload
        .get(&meta.pk_column)
        .or_else(|| payload.get("id"))
        .cloned();

    let generated_str_id = if !meta.pk_is_auto_increment && custom_id.is_none() {
        Some(generate_id())
    } else {
        None
    };

    if let Some(ref gid) = generated_str_id {
        payload.insert(meta.pk_column.clone(), Value::String(gid.clone()));
        if meta.pk_column != "id" && meta.has_column("id") {
            payload.insert("id".to_string(), Value::String(gid.clone()));
        }
    }

    if meta.has_created && !payload.contains_key("created") {
        payload.insert("created".to_string(), Value::String(now.clone()));
    }
    if meta.has_updated && !payload.contains_key("updated") {
        payload.insert("updated".to_string(), Value::String(now.clone()));
    }
    if meta.has_created_at && !payload.contains_key("created_at") {
        payload.insert("created_at".to_string(), resolve_timestamp_val(&meta, "created_at", &now));
    }
    if meta.has_updated_at && !payload.contains_key("updated_at") {
        payload.insert("updated_at".to_string(), resolve_timestamp_val(&meta, "updated_at", &now));
    }

    if col.collection_type == CollectionType::Auth {
        if let Some(pass_val) = payload.remove("password") {
            if let Some(pass_str) = pass_val.as_str() {
                let hash = crate::core::auth::hash_password(pass_str)?;
                if meta.has_column("passwordHash") {
                    payload.insert("passwordHash".to_string(), Value::String(hash.clone()));
                }
                if meta.has_column("password") {
                    payload.insert("password".to_string(), Value::String(hash));
                }
            }
        }
        if meta.has_column("tokenKey") && !payload.contains_key("tokenKey") {
            payload.insert(
                "tokenKey".to_string(),
                Value::String(crate::core::auth::generate_token_key()),
            );
        }

        if meta.has_column("role") && !payload.contains_key("role") {
            payload.insert("role".to_string(), Value::String("user".to_string()));
        }
        if meta.has_column("status") && !payload.contains_key("status") {
            payload.insert("status".to_string(), Value::String("active".to_string()));
        }

        // Auto-generate username if not provided
        let has_username = payload
            .get("username")
            .and_then(|u| u.as_str())
            .map(|s| !s.trim().is_empty())
            .unwrap_or(false);
        if !has_username && meta.has_column("username") {
            let default_username =
                if let Some(email) = payload.get("email").and_then(|e| e.as_str()) {
                    email.split('@').next().unwrap_or("user").to_string()
                } else {
                    format!("user_{}", Utc::now().timestamp_subsec_millis())
                };
            payload.insert("username".to_string(), Value::String(default_username));
        }
    }

    let mut keys = Vec::new();
    let mut placeholders = Vec::new();
    let mut params = Vec::new();

    for (k, v) in &payload {
        // Lewati primary key jika auto-increment dan nilainya kosong/null
        if k.eq_ignore_ascii_case(&meta.pk_column) && meta.pk_is_auto_increment {
            let is_empty = v.is_null()
                || v.as_str().map(|s| s.trim().is_empty()).unwrap_or(false)
                || v.as_i64() == Some(0);
            if is_empty {
                continue;
            }
        }
        if !meta.has_column(k) {
            continue;
        }
        keys.push(format!("`{}`", k));
        placeholders.push("?");
        params.push(Val::from(v));
    }

    if pool.is_postgres() {
        let pg_keys = keys.join(", ").replace('`', "\"");
        let sql = format!(
            "INSERT INTO \"{}\" ({}) VALUES ({}) RETURNING *;",
            col.name,
            pg_keys,
            placeholders.join(", ")
        );
        let row = pool.fetch_one(&sql, &params).await?;
        return json_row_to_record(&row, col, &meta);
    }

    let sql = format!(
        "INSERT INTO `{}` ({}) VALUES ({});",
        col.name,
        keys.join(", "),
        placeholders.join(", ")
    );

    let (_, last_id) = pool.execute(&sql, &params).await?;

    let lookup_id = if let Some(ref gid) = generated_str_id {
        gid.clone()
    } else if let Some(ref cid) = custom_id {
        if let Some(s) = cid.as_str() {
            s.to_string()
        } else if let Some(n) = cid.as_i64() {
            n.to_string()
        } else {
            cid.to_string()
        }
    } else if let Some(lid) = last_id {
        lid.to_string()
    } else {
        return Err(anyhow!("Record created but could not resolve primary key"));
    };

    get_record(pool, col, &lookup_id)
        .await?
        .ok_or_else(|| anyhow!("Record was not created"))
}

pub async fn get_record(pool: &DbPool, col: &Collection, id: &str) -> Result<Option<Record>> {
    get_record_with(pool, col, id, None).await
}

pub async fn get_record_with(
    pool: &DbPool,
    col: &Collection,
    id: &str,
    with_relations: Option<&[&str]>,
) -> Result<Option<Record>> {
    let meta = super::db::get_table_metadata(pool, &col.name).await?;
    let sql = format!(
        "SELECT * FROM `{}` WHERE `{}` = ? LIMIT 1;",
        col.name, meta.pk_column
    );
    let row_opt = pool.fetch_optional(&sql, &[pk_to_val(id, &meta)]).await?;

    if let Some(row) = row_opt {
        let rec = json_row_to_record(&row, col, &meta)?;
        let mut items = vec![rec];
        let _ = eager_load_relations(pool, &col.name, col.auto_load_relations, &mut items, with_relations).await;
        Ok(items.pop())
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
    let meta = super::db::get_table_metadata(pool, &col.name).await?;
    let now = Utc::now().to_rfc3339();

    payload.remove(&meta.pk_column);
    payload.remove("id");
    payload.remove("created");
    payload.remove("created_at");
    payload.remove("created_by");
    payload.remove("collectionId");
    payload.remove("collectionName");

    if meta.has_updated {
        payload.insert("updated".to_string(), Value::String(now.clone()));
    }
    if meta.has_updated_at {
        payload.insert("updated_at".to_string(), resolve_timestamp_val(&meta, "updated_at", &now));
    }

    if col.collection_type == CollectionType::Auth {
        if let Some(pass_val) = payload.remove("password") {
            if let Some(pass_str) = pass_val.as_str() {
                if !pass_str.trim().is_empty() {
                    let hash = crate::core::auth::hash_password(pass_str)?;
                    if meta.has_column("passwordHash") {
                        payload.insert("passwordHash".to_string(), Value::String(hash.clone()));
                    }
                    if meta.has_column("password") {
                        payload.insert("password".to_string(), Value::String(hash));
                    }
                }
            }
        }
        if let Some(v) = payload.get("verified") {
            let is_verified = match v {
                Value::Bool(b) => *b,
                Value::Number(n) => n.as_i64().unwrap_or(0) == 1,
                Value::String(s) => s == "1" || s.eq_ignore_ascii_case("true"),
                _ => false,
            };
            if is_verified {
                if meta.has_column("email_verified_at") && !payload.contains_key("email_verified_at") {
                    payload.insert(
                        "email_verified_at".to_string(),
                        Value::Number(chrono::Utc::now().timestamp_millis().into()),
                    );
                }
            } else if meta.has_column("email_verified_at") && !payload.contains_key("email_verified_at") {
                payload.insert("email_verified_at".to_string(), Value::Null);
            }
        }
    }

    let mut set_clauses = Vec::new();
    let mut params = Vec::new();

    for (k, v) in &payload {
        if !meta.has_column(k) {
            continue;
        }
        set_clauses.push(format!("`{}` = ?", k));
        params.push(Val::from(v));
    }
    params.push(pk_to_val(id, &meta));

    let sql = format!(
        "UPDATE `{}` SET {} WHERE `{}` = ?;",
        col.name,
        set_clauses.join(", "),
        meta.pk_column
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
    let meta = super::db::get_table_metadata(pool, &col.name).await?;
    let sql = format!("DELETE FROM `{}` WHERE `{}` = ?;", col.name, meta.pk_column);
    let (rows_affected, _) = pool.execute(&sql, &[pk_to_val(id, &meta)]).await?;
    Ok(rows_affected > 0)
}

pub async fn get_record_tx(
    tx: &mut DbTx<'_>,
    pool: &DbPool,
    col: &Collection,
    id: &str,
) -> Result<Option<Record>> {
    let meta = super::db::get_table_metadata(pool, &col.name).await?;
    let sql = format!(
        "SELECT * FROM `{}` WHERE `{}` = ? LIMIT 1;",
        col.name, meta.pk_column
    );
    let row_opt = tx.fetch_optional(pool, &sql, &[pk_to_val(id, &meta)]).await?;
    if let Some(row) = row_opt {
        Ok(Some(json_row_to_record(&row, col, &meta)?))
    } else {
        Ok(None)
    }
}

pub async fn create_record_tx(
    tx: &mut DbTx<'_>,
    pool: &DbPool,
    col: &Collection,
    mut payload: Map<String, Value>,
) -> Result<Record> {
    let meta = super::db::get_table_metadata(pool, &col.name).await?;
    let now = Utc::now().to_rfc3339();

    apply_schema_defaults(col, &mut payload);

    let custom_id = payload
        .get(&meta.pk_column)
        .or_else(|| payload.get("id"))
        .cloned();

    let generated_str_id = if !meta.pk_is_auto_increment && custom_id.is_none() {
        Some(generate_id())
    } else {
        None
    };

    if let Some(ref gid) = generated_str_id {
        payload.insert(meta.pk_column.clone(), Value::String(gid.clone()));
        if meta.pk_column != "id" && meta.has_column("id") {
            payload.insert("id".to_string(), Value::String(gid.clone()));
        }
    }

    if meta.has_created && !payload.contains_key("created") {
        payload.insert("created".to_string(), Value::String(now.clone()));
    }
    if meta.has_updated && !payload.contains_key("updated") {
        payload.insert("updated".to_string(), Value::String(now.clone()));
    }
    if meta.has_created_at && !payload.contains_key("created_at") {
        payload.insert("created_at".to_string(), resolve_timestamp_val(&meta, "created_at", &now));
    }
    if meta.has_updated_at && !payload.contains_key("updated_at") {
        payload.insert("updated_at".to_string(), resolve_timestamp_val(&meta, "updated_at", &now));
    }

    if col.collection_type == CollectionType::Auth {
        if let Some(pass_val) = payload.remove("password") {
            if let Some(pass_str) = pass_val.as_str() {
                let hash = crate::core::auth::hash_password(pass_str)?;
                if meta.has_column("passwordHash") {
                    payload.insert("passwordHash".to_string(), Value::String(hash.clone()));
                }
                if meta.has_column("password") {
                    payload.insert("password".to_string(), Value::String(hash));
                }
            }
        }
        if meta.has_column("tokenKey") && !payload.contains_key("tokenKey") {
            payload.insert(
                "tokenKey".to_string(),
                Value::String(crate::core::auth::generate_token_key()),
            );
        }
        if meta.has_column("role") && !payload.contains_key("role") {
            payload.insert("role".to_string(), Value::String("user".to_string()));
        }
        if meta.has_column("status") && !payload.contains_key("status") {
            payload.insert("status".to_string(), Value::String("active".to_string()));
        }
        let has_username = payload
            .get("username")
            .and_then(|u| u.as_str())
            .map(|s| !s.trim().is_empty())
            .unwrap_or(false);
        if !has_username && meta.has_column("username") {
            let default_username =
                if let Some(email) = payload.get("email").and_then(|e| e.as_str()) {
                    email.split('@').next().unwrap_or("user").to_string()
                } else {
                    format!("user_{}", Utc::now().timestamp_subsec_millis())
                };
            payload.insert("username".to_string(), Value::String(default_username));
        }
    }

    let mut keys = Vec::new();
    let mut placeholders = Vec::new();
    let mut params = Vec::new();

    for (k, v) in &payload {
        if k.eq_ignore_ascii_case(&meta.pk_column) && meta.pk_is_auto_increment {
            let is_empty = v.is_null()
                || v.as_str().map(|s| s.trim().is_empty()).unwrap_or(false)
                || v.as_i64() == Some(0);
            if is_empty {
                continue;
            }
        }
        if !meta.has_column(k) {
            continue;
        }
        keys.push(format!("`{}`", k));
        placeholders.push("?");
        params.push(Val::from(v));
    }

    if pool.is_postgres() {
        let pg_keys = keys.join(", ").replace('`', "\"");
        let sql = format!(
            "INSERT INTO \"{}\" ({}) VALUES ({}) RETURNING *;",
            col.name,
            pg_keys,
            placeholders.join(", ")
        );
        let row = tx.fetch_one(pool, &sql, &params).await?;
        return json_row_to_record(&row, col, &meta);
    }

    let sql = format!(
        "INSERT INTO `{}` ({}) VALUES ({});",
        col.name,
        keys.join(", "),
        placeholders.join(", ")
    );

    let (_, last_id) = tx.execute(pool, &sql, &params).await?;

    let lookup_id = if let Some(ref gid) = generated_str_id {
        gid.clone()
    } else if let Some(ref cid) = custom_id {
        if let Some(s) = cid.as_str() {
            s.to_string()
        } else if let Some(n) = cid.as_i64() {
            n.to_string()
        } else {
            cid.to_string()
        }
    } else if let Some(lid) = last_id {
        lid.to_string()
    } else {
        return Err(anyhow!("Record created in transaction but could not resolve primary key"));
    };

    get_record_tx(tx, pool, col, &lookup_id)
        .await?
        .ok_or_else(|| anyhow!("Record was not created in transaction"))
}

pub async fn update_record_tx(
    tx: &mut DbTx<'_>,
    pool: &DbPool,
    col: &Collection,
    id: &str,
    mut payload: Map<String, Value>,
) -> Result<Record> {
    let meta = super::db::get_table_metadata(pool, &col.name).await?;
    let now = Utc::now().to_rfc3339();

    payload.remove(&meta.pk_column);
    payload.remove("id");
    payload.remove("created");
    payload.remove("created_at");
    payload.remove("created_by");
    payload.remove("collectionId");
    payload.remove("collectionName");

    if meta.has_updated {
        payload.insert("updated".to_string(), Value::String(now.clone()));
    }
    if meta.has_updated_at {
        payload.insert("updated_at".to_string(), resolve_timestamp_val(&meta, "updated_at", &now));
    }

    if col.collection_type == CollectionType::Auth {
        if let Some(pass_val) = payload.remove("password") {
            if let Some(pass_str) = pass_val.as_str() {
                if !pass_str.trim().is_empty() {
                    let hash = crate::core::auth::hash_password(pass_str)?;
                    if meta.has_column("passwordHash") {
                        payload.insert("passwordHash".to_string(), Value::String(hash.clone()));
                    }
                    if meta.has_column("password") {
                        payload.insert("password".to_string(), Value::String(hash));
                    }
                }
            }
        }
        if let Some(v) = payload.get("verified") {
            let is_verified = match v {
                Value::Bool(b) => *b,
                Value::Number(n) => n.as_i64().unwrap_or(0) == 1,
                Value::String(s) => s == "1" || s.eq_ignore_ascii_case("true"),
                _ => false,
            };
            if is_verified {
                if meta.has_column("email_verified_at") && !payload.contains_key("email_verified_at") {
                    payload.insert(
                        "email_verified_at".to_string(),
                        Value::Number(chrono::Utc::now().timestamp_millis().into()),
                    );
                }
            } else if meta.has_column("email_verified_at") && !payload.contains_key("email_verified_at") {
                payload.insert("email_verified_at".to_string(), Value::Null);
            }
        }
    }

    let mut set_clauses = Vec::new();
    let mut params = Vec::new();

    for (k, v) in &payload {
        if !meta.has_column(k) {
            continue;
        }
        set_clauses.push(format!("`{}` = ?", k));
        params.push(Val::from(v));
    }
    params.push(pk_to_val(id, &meta));

    let sql = format!(
        "UPDATE `{}` SET {} WHERE `{}` = ?;",
        col.name,
        set_clauses.join(", "),
        meta.pk_column
    );

    let (rows_affected, _) = tx.execute(pool, &sql, &params).await?;
    if rows_affected == 0 {
        return Err(anyhow!("Record not found: {}", id));
    }

    get_record_tx(tx, pool, col, id)
        .await?
        .ok_or_else(|| anyhow!("Record not found after update in transaction"))
}

pub async fn delete_record_tx(
    tx: &mut DbTx<'_>,
    pool: &DbPool,
    col: &Collection,
    id: &str,
) -> Result<bool> {
    let meta = super::db::get_table_metadata(pool, &col.name).await?;
    let sql = format!("DELETE FROM `{}` WHERE `{}` = ?;", col.name, meta.pk_column);
    let (rows_affected, _) = tx.execute(pool, &sql, &[pk_to_val(id, &meta)]).await?;
    Ok(rows_affected > 0)
}

/// Columns that must never be referenced from a client supplied filter. Allowing them would turn
/// `filter=password~'a'` into a blind oracle that leaks hashes / session keys character by character.
const HIDDEN_FILTER_COLUMNS: &[&str] = &["password", "passwordhash", "tokenkey", "remember_token"];

/// SQL keywords that can never appear as a bare identifier in a filter (subqueries, DDL, DML, ...).
const DENIED_FILTER_WORDS: &[&str] = &[
    "select", "from", "where", "union", "join", "exists", "case", "when", "then", "else", "end",
    "group", "having", "order", "limit", "offset", "with", "as", "intersect", "except", "insert",
    "update", "delete", "drop", "alter", "create", "attach", "detach", "pragma", "values", "set",
    "replace", "truncate", "exec", "execute", "call", "collate", "returning",
];

/// Convert a PocketBase-style filter into a SQL boolean expression using a strict whitelist
/// tokenizer. Only column names, literals, comparison operators, `AND`/`OR`/`NOT`, `LIKE`, `IS NULL`,
/// `IN (...)`, `BETWEEN` and parentheses survive. Function calls, sub-selects, comments, statement
/// separators, backslashes and cross-table references are rejected (`None`).
fn tokenize_filter(f: &str) -> Option<String> {
    if f.len() > 1000 {
        return None;
    }
    let chars: Vec<char> = f.chars().collect();
    let n = chars.len();
    let mut i = 0;
    let mut out: Vec<String> = Vec::new();

    while i < n {
        let c = chars[i];
        if c.is_whitespace() {
            i += 1;
            continue;
        }
        if out.len() > 120 {
            return None;
        }
        match c {
            '(' | ')' | ',' => {
                out.push(c.to_string());
                i += 1;
            }
            '\'' | '"' => {
                let quote = c;
                i += 1;
                let mut s = String::new();
                let mut closed = false;
                while i < n {
                    let d = chars[i];
                    if d == quote {
                        if i + 1 < n && chars[i + 1] == quote {
                            s.push(quote);
                            i += 2;
                            continue;
                        }
                        closed = true;
                        i += 1;
                        break;
                    }
                    // Backslash is an escape character on MySQL: refuse it outright.
                    if d == '\\' || d == '\0' {
                        return None;
                    }
                    s.push(d);
                    i += 1;
                    if s.len() > 256 {
                        return None;
                    }
                }
                if !closed {
                    return None;
                }
                out.push(format!("'{}'", s.replace('\'', "''")));
            }
            '&' => {
                if i + 1 < n && chars[i + 1] == '&' {
                    out.push("AND".to_string());
                    i += 2;
                } else {
                    return None;
                }
            }
            '|' => {
                if i + 1 < n && chars[i + 1] == '|' {
                    out.push("OR".to_string());
                    i += 2;
                } else {
                    return None;
                }
            }
            '=' => {
                out.push("=".to_string());
                i += 1;
            }
            '!' => {
                if i + 1 < n && chars[i + 1] == '=' {
                    out.push("!=".to_string());
                    i += 2;
                } else if i + 1 < n && chars[i + 1] == '~' {
                    out.push("NOT LIKE".to_string());
                    i += 2;
                } else {
                    return None;
                }
            }
            '<' => {
                if i + 1 < n && chars[i + 1] == '=' {
                    out.push("<=".to_string());
                    i += 2;
                } else if i + 1 < n && chars[i + 1] == '>' {
                    out.push("<>".to_string());
                    i += 2;
                } else {
                    out.push("<".to_string());
                    i += 1;
                }
            }
            '>' => {
                if i + 1 < n && chars[i + 1] == '=' {
                    out.push(">=".to_string());
                    i += 2;
                } else {
                    out.push(">".to_string());
                    i += 1;
                }
            }
            '~' => {
                out.push("LIKE".to_string());
                i += 1;
            }
            '-' | '0'..='9' => {
                let start = i;
                if c == '-' {
                    i += 1;
                    if i >= n || !chars[i].is_ascii_digit() {
                        return None; // `--` comments and stray minus signs are not allowed
                    }
                }
                while i < n && (chars[i].is_ascii_digit() || chars[i] == '.') {
                    i += 1;
                }
                out.push(chars[start..i].iter().collect());
            }
            c if c.is_ascii_alphabetic() || c == '_' => {
                let start = i;
                while i < n && (chars[i].is_ascii_alphanumeric() || chars[i] == '_') {
                    i += 1;
                }
                let word: String = chars[start..i].iter().collect();
                let lower = word.to_ascii_lowercase();
                // `table.column` style references are not allowed
                if i < n && chars[i] == '.' {
                    return None;
                }
                let mut j = i;
                while j < n && chars[j].is_whitespace() {
                    j += 1;
                }
                let followed_by_paren = j < n && chars[j] == '(';

                match lower.as_str() {
                    "and" | "or" | "not" | "like" | "is" | "null" | "in" | "between" => {
                        out.push(lower.to_ascii_uppercase())
                    }
                    "true" => out.push("1".to_string()),
                    "false" => out.push("0".to_string()),
                    _ => {
                        if followed_by_paren
                            || word.len() > 64
                            || DENIED_FILTER_WORDS.contains(&lower.as_str())
                            || HIDDEN_FILTER_COLUMNS.contains(&lower.as_str())
                        {
                            return None;
                        }
                        out.push(format!("`{}`", word));
                    }
                }
            }
            _ => return None,
        }
    }

    if out.is_empty() {
        None
    } else {
        Some(out.join(" "))
    }
}

fn parse_pocketbase_filter(raw: &str, col: &Collection) -> Result<String> {
    let f = raw.trim();
    if f.is_empty() {
        return Ok(String::new());
    }

    let has_operator = f.contains('=')
        || f.contains('~')
        || f.contains('>')
        || f.contains('<')
        || f.to_uppercase().contains(" LIKE ")
        || f.to_uppercase().contains(" IS ")
        || f.to_uppercase().contains(" IN ");

    if !has_operator {
        // Quick keyword search across id and text columns. The keyword is reduced to a harmless
        // literal: no backslashes / NULs, quotes doubled, LIKE wildcards escaped with `!`.
        let kw: String = f
            .chars()
            .filter(|c| *c != '\\' && *c != '\0')
            .take(100)
            .collect();
        let safe_kw = kw
            .replace('!', "!!")
            .replace('%', "!%")
            .replace('_', "!_")
            .replace('\'', "''");
        let like = |column: &str| format!("`{}` LIKE '%{}%' ESCAPE '!'", column, safe_kw);
        let mut clauses = vec![like("id")];
        if col.collection_type == CollectionType::Auth {
            clauses.push(like("email"));
            clauses.push(like("username"));
        }
        for field in &col.schema {
            if matches!(
                field.field_type,
                FieldType::Text | FieldType::Email | FieldType::Url | FieldType::Select
            ) && super::db::is_safe_identifier(&field.name)
                && !HIDDEN_FILTER_COLUMNS.contains(&field.name.to_ascii_lowercase().as_str())
            {
                clauses.push(like(&field.name));
            }
        }
        return Ok(format!("WHERE ({})", clauses.join(" OR ")));
    }

    match tokenize_filter(f) {
        Some(expr) => Ok(format!("WHERE {}", expr)),
        // Anything outside the whitelist matches nothing instead of reaching the database.
        None => Ok("WHERE 1 = 0".to_string()),
    }
}

fn build_filter_and_sort_sql(
    meta: &TableMetadata,
    col: &Collection,
    sort: Option<&str>,
    filter: Option<&str>,
) -> Result<(String, String)> {
    let filter_sql = match filter {
        Some(f) => parse_pocketbase_filter(f, col)?,
        None => String::new(),
    };

    let default_sort = if meta.has_created {
        "ORDER BY `created` DESC".to_string()
    } else if meta.has_created_at {
        "ORDER BY `created_at` DESC".to_string()
    } else {
        format!("ORDER BY `{}` DESC", meta.pk_column)
    };

    let sort_sql = if let Some(s) = sort {
        if !s.trim().is_empty() {
            let mut valid_parts = Vec::new();
            for item in s.split(',') {
                let item = item.trim();
                let (raw_col, is_desc) = if let Some(stripped) = item.strip_prefix('-') {
                    (stripped, true)
                } else if let Some(stripped) = item.strip_prefix('+') {
                    (stripped, false)
                } else {
                    (item, false)
                };

                let target_col = if meta.has_column(raw_col) {
                    Some(raw_col.to_string())
                } else if raw_col.eq_ignore_ascii_case("created") && meta.has_created_at {
                    Some("created_at".to_string())
                } else if raw_col.eq_ignore_ascii_case("updated") && meta.has_updated_at {
                    Some("updated_at".to_string())
                } else if raw_col.eq_ignore_ascii_case("id") && meta.pk_column != "id" {
                    Some(meta.pk_column.clone())
                } else {
                    None
                };

                if let Some(c) = target_col {
                    if is_desc {
                        valid_parts.push(format!("`{}` DESC", c));
                    } else {
                        valid_parts.push(format!("`{}` ASC", c));
                    }
                }
            }

            if !valid_parts.is_empty() {
                format!("ORDER BY {}", valid_parts.join(", "))
            } else {
                default_sort
            }
        } else {
            default_sort
        }
    } else {
        default_sort
    };

    Ok((filter_sql, sort_sql))
}

pub async fn eager_load_relations(
    pool: &DbPool,
    table_name: &str,
    auto_load_default: bool,
    records: &mut [Record],
    with_relations: Option<&[&str]>,
) -> Result<()> {
    if records.is_empty() {
        return Ok(());
    }

    if let Some(with_list) = with_relations {
        if with_list.iter().any(|w| w.eq_ignore_ascii_case("none") || w.eq_ignore_ascii_case("false")) {
            return Ok(());
        }
    } else {
        // Cek global option dari .env (AUTO_LOAD_RELATIONS) secara lock-free.
        // Jika true: semua relasi di-load secara eager loading.
        // Jika false: fallback ke pengaturan per-tabel di dashboard (auto_load_default).
        let global_auto_load = crate::core::env::Env::auto_load_relations();

        if !global_auto_load && !auto_load_default {
            return Ok(());
        }
    }

    let all_relations = super::db::get_table_relations(pool, table_name).await?;
    if all_relations.is_empty() {
        return Ok(());
    }

    let target_relations: Vec<&super::db::DetectedRelation> = if let Some(with_list) = with_relations {
        if with_list.iter().any(|w| *w == "*" || w.eq_ignore_ascii_case("all")) {
            all_relations.iter().collect()
        } else {
            all_relations
                .iter()
                .filter(|r| {
                    with_list.iter().any(|w| {
                        w.eq_ignore_ascii_case(&r.relation_name) || w.eq_ignore_ascii_case(&r.column)
                    })
                })
                .collect()
        }
    } else {
        all_relations.iter().collect()
    };

    for rel in target_relations {
        let mut ids = Vec::new();
        let mut seen = std::collections::HashSet::new();

        for rec in records.iter() {
            let val_opt = rec.data.get(&rel.column).or_else(|| rec.data.get(&rel.relation_name));
            if let Some(val) = val_opt {
                if val.is_null() {
                    continue;
                }
                let id_str = match val {
                    Value::String(s) => s.trim().to_string(),
                    Value::Number(n) => n.to_string(),
                    _ => String::new(),
                };
                if !id_str.is_empty() && id_str != "0" && seen.insert(id_str.clone()) {
                    if let Ok(num) = id_str.parse::<i64>() {
                        ids.push(Val::Int(num));
                    } else {
                        ids.push(Val::from(id_str));
                    }
                }
            }
        }

        if ids.is_empty() {
            continue;
        }

        let placeholders = vec!["?"; ids.len()].join(", ");
        let cols_select = if rel.display_col == rel.target_pk {
            format!("`{}`", rel.target_pk)
        } else {
            format!("`{}`, `{}`", rel.target_pk, rel.display_col)
        };

        let sql = format!(
            "SELECT {} FROM `{}` WHERE `{}` IN ({});",
            cols_select, rel.target_table, rel.target_pk, placeholders
        );

        let rows = pool.fetch_all(&sql, &ids).await.unwrap_or_default();

        let mut map: std::collections::HashMap<String, Value> = std::collections::HashMap::new();
        for row in rows {
            let pk_val = json_get_str(&row, &rel.target_pk);
            let pk_val = if pk_val.is_empty() {
                json_get_str(&row, "id")
            } else {
                pk_val
            };

            let mut rel_obj = Map::new();
            if let Some(obj) = row.as_object() {
                for (k, v) in obj {
                    rel_obj.insert(k.clone(), v.clone());
                }
            }
            if !rel_obj.contains_key("id") && !pk_val.is_empty() {
                rel_obj.insert("id".to_string(), Value::String(pk_val.clone()));
            }

            map.insert(pk_val, Value::Object(rel_obj));
        }

        for rec in records.iter_mut() {
            let fk_val = rec.data.get(&rel.column).cloned();
            if let Some(val) = fk_val {
                let id_str = match val {
                    Value::String(ref s) => s.trim().to_string(),
                    Value::Number(ref n) => n.to_string(),
                    _ => String::new(),
                };
                if let Some(related_data) = map.get(&id_str) {
                    rec.data.insert(rel.relation_name.clone(), related_data.clone());
                } else if !id_str.is_empty() && id_str != "0" {
                    rec.data.insert(rel.relation_name.clone(), Value::Null);
                }
            }
        }
    }

    Ok(())
}

pub async fn get_all_records(
    pool: &DbPool,
    col: &Collection,
    sort: Option<&str>,
    filter: Option<&str>,
) -> Result<Vec<Record>> {
    get_all_records_with(pool, col, sort, filter, None).await
}

pub async fn get_all_records_with(
    pool: &DbPool,
    col: &Collection,
    sort: Option<&str>,
    filter: Option<&str>,
    with_relations: Option<&[&str]>,
) -> Result<Vec<Record>> {
    get_all_records_with_limit(pool, col, sort, filter, with_relations, None, None).await
}

pub async fn get_all_records_with_limit(
    pool: &DbPool,
    col: &Collection,
    sort: Option<&str>,
    filter: Option<&str>,
    with_relations: Option<&[&str]>,
    limit: Option<usize>,
    offset: Option<usize>,
) -> Result<Vec<Record>> {
    let meta = super::db::get_table_metadata(pool, &col.name).await?;
    let (filter_sql, sort_sql) = build_filter_and_sort_sql(&meta, col, sort, filter)?;

    let limit_clause = match limit {
        Some(lim) => {
            if let Some(off) = offset {
                format!("LIMIT {} OFFSET {}", lim, off)
            } else {
                format!("LIMIT {}", lim)
            }
        }
        None => {
            if let Some(off) = offset {
                format!("LIMIT 5000 OFFSET {}", off)
            } else {
                String::new()
            }
        }
    };

    let sql = format!("SELECT * FROM `{}` {} {} {};", col.name, filter_sql, sort_sql, limit_clause);
    let rows = pool.fetch_all(&sql, &[]).await?;

    let mut items = Vec::new();
    for row in rows {
        items.push(json_row_to_record(&row, col, &meta)?);
    }

    let _ = eager_load_relations(pool, &col.name, col.auto_load_relations, &mut items, with_relations).await;

    Ok(items)
}

pub async fn list_records(
    pool: &DbPool,
    col: &Collection,
    page: i64,
    per_page: i64,
    sort: Option<&str>,
    filter: Option<&str>,
) -> Result<PaginatedRecords> {
    list_records_with(pool, col, page, per_page, sort, filter, None).await
}

pub async fn list_records_with(
    pool: &DbPool,
    col: &Collection,
    page: i64,
    per_page: i64,
    sort: Option<&str>,
    filter: Option<&str>,
    with_relations: Option<&[&str]>,
) -> Result<PaginatedRecords> {
    if per_page <= 0 {
        let items = get_all_records_with(pool, col, sort, filter, with_relations).await?;
        let total_items = items.len() as i64;
        return Ok(PaginatedRecords {
            page: 1,
            per_page: total_items,
            total_items,
            total_pages: 1,
            items,
        });
    }

    let page = if page < 1 { 1 } else { page };
    let offset = (page - 1) * per_page;

    let meta = super::db::get_table_metadata(pool, &col.name).await?;
    let (filter_sql, sort_sql) = build_filter_and_sort_sql(&meta, col, sort, filter)?;

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
        items.push(json_row_to_record(&row, col, &meta)?);
    }

    let _ = eager_load_relations(pool, &col.name, col.auto_load_relations, &mut items, with_relations).await;

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

fn json_row_to_record(row: &Value, col: &Collection, meta: &TableMetadata) -> Result<Record> {
    let mut data = Map::new();
    let id_str = json_get_str(row, &meta.pk_column);
    let id_str = if id_str.is_empty() {
        json_get_str(row, "id")
    } else {
        id_str
    };

    if let Some(obj) = row.as_object() {
        for (raw_name, raw_val) in obj {
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

    if !data.contains_key("id") && !id_str.is_empty() {
        data.insert("id".to_string(), Value::String(id_str.clone()));
    }

    Ok(Record {
        id: id_str,
        data,
    })
}

/// Count records matching an optional filter (compatible with Padi Framework `count` query).
pub async fn count_records(
    pool: &DbPool,
    col: &Collection,
    filter: Option<&str>,
) -> Result<i64> {
    let meta = super::db::get_table_metadata(pool, &col.name).await?;
    let (filter_sql, _) = build_filter_and_sort_sql(&meta, col, None, filter)?;
    let count_sql = format!(
        "SELECT COUNT(*) as `total` FROM `{}` {};",
        col.name, filter_sql
    );
    let total_row = pool.fetch_one(&count_sql, &[]).await?;
    Ok(json_get_i64(&total_row, "total"))
}

/// Delete multiple records in a single batch query using `WHERE pk IN (?, ...)` (ported from Padi Framework).
pub async fn delete_records_batch(
    pool: &DbPool,
    col: &Collection,
    ids: &[String],
) -> Result<u64> {
    if ids.is_empty() {
        return Ok(0);
    }
    let meta = super::db::get_table_metadata(pool, &col.name).await?;
    let placeholders = vec!["?"; ids.len()].join(", ");
    let params: Vec<Val> = ids.iter().map(|id| pk_to_val(id, &meta)).collect();
    let sql = format!(
        "DELETE FROM `{}` WHERE `{}` IN ({});",
        col.name, meta.pk_column, placeholders
    );
    let (rows_affected, _) = pool.execute(&sql, &params).await?;
    Ok(rows_affected)
}

/// Filter record fields in-place when `?fields=id,name` or `?select=id,name` is requested.
pub fn project_record_fields(rec: &mut Record, fields: Option<&[&str]>) {
    let Some(field_list) = fields else {
        return;
    };
    if field_list.is_empty() || field_list.iter().any(|f| *f == "*") {
        return;
    }
    rec.data.retain(|k, _| {
        k.eq_ignore_ascii_case("id")
            || field_list.iter().any(|f| f.eq_ignore_ascii_case(k))
    });
}
