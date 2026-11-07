//! Multi-Database Engine for padiBase (SQLite, MySQL, MariaDB, PostgreSQL)
//! Inspired by and compatible with Padi Rust Api Framework (`padi_core::database`).

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use sqlx::{
    mysql::MySqlPoolOptions,
    postgres::PgPoolOptions,
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous},
    Column, Row,
};
use std::collections::HashMap;
use std::path::Path;
use std::str::FromStr;
use std::sync::{Arc, RwLock};
use tracing::info;

use super::env::Env;
use super::schema::{generate_id, Collection, CollectionType, CustomEndpoint, FieldType, SchemaField};

/// Tenant/Database Schema Cache - populated once at startup, served entirely from RAM.
#[derive(Default, Debug)]
pub struct SchemaCache {
    pub table_metadata: RwLock<HashMap<String, TableMetadata>>,
    pub relations: RwLock<HashMap<String, Vec<DetectedRelation>>>,
    pub collections: RwLock<HashMap<String, Collection>>,
    pub collections_list: RwLock<Option<Vec<Collection>>>,
    pub endpoints: RwLock<HashMap<String, CustomEndpoint>>,
    pub settings: RwLock<Option<serde_json::Value>>,
}

/// Validate that a SQL identifier (table or column name) only contains safe alphanumeric/underscore characters.
#[inline]
pub fn is_safe_identifier(name: &str) -> bool {
    let trimmed = name.trim();
    !trimmed.is_empty()
        && trimmed.len() <= 128
        && trimmed
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

/// Standalone fallback invalidation helpers (no-op as cache is tenant/pool isolated).
pub fn invalidate_schema_cache(_table_or_col_name: &str) {}
pub fn clear_all_schema_cache() {}

/// Supported database drivers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DbDriver {
    Sqlite,
    MySql,
    MariaDb,
    Postgres,
}

impl DbDriver {
    pub fn as_str(&self) -> &'static str {
        match self {
            DbDriver::Sqlite => "sqlite",
            DbDriver::MySql => "mysql",
            DbDriver::MariaDb => "mariadb",
            DbDriver::Postgres => "postgres",
        }
    }
}

/// Physical driver connection pool enum.
#[derive(Clone, Debug)]
pub enum DbDriverPool {
    Sqlite(sqlx::SqlitePool),
    MySql(sqlx::MySqlPool, DbDriver),
    Postgres(sqlx::PgPool),
}

/// Unified multi-database connection pool with isolated in-memory tenant schema cache.
#[derive(Clone, Debug)]
pub struct DbPool {
    pub driver_pool: DbDriverPool,
    pub cache: Arc<SchemaCache>,
}

/// A bindable SQL parameter value (compatible with `padi_core::database::Val`).
#[derive(Debug, Clone, PartialEq)]
pub enum Val {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    Str(String),
}

impl From<&str> for Val {
    fn from(v: &str) -> Self {
        Val::Str(v.into())
    }
}
impl From<String> for Val {
    fn from(v: String) -> Self {
        Val::Str(v)
    }
}
impl From<&String> for Val {
    fn from(v: &String) -> Self {
        Val::Str(v.clone())
    }
}
impl From<i64> for Val {
    fn from(v: i64) -> Self {
        Val::Int(v)
    }
}
impl From<i32> for Val {
    fn from(v: i32) -> Self {
        Val::Int(v as i64)
    }
}
impl From<usize> for Val {
    fn from(v: usize) -> Self {
        Val::Int(v as i64)
    }
}
impl From<f64> for Val {
    fn from(v: f64) -> Self {
        Val::Float(v)
    }
}
impl From<bool> for Val {
    fn from(v: bool) -> Self {
        Val::Bool(v)
    }
}
impl<T: Into<Val>> From<Option<T>> for Val {
    fn from(v: Option<T>) -> Self {
        v.map(Into::into).unwrap_or(Val::Null)
    }
}
impl From<&Value> for Val {
    fn from(v: &Value) -> Self {
        match v {
            Value::Null => Val::Null,
            Value::Bool(b) => Val::Bool(*b),
            Value::Number(n) => n
                .as_i64()
                .map(Val::Int)
                .unwrap_or_else(|| Val::Float(n.as_f64().unwrap_or(0.0))),
            Value::String(s) => Val::Str(s.clone()),
            other => Val::Str(other.to_string()),
        }
    }
}
impl From<Value> for Val {
    fn from(v: Value) -> Self {
        (&v).into()
    }
}

impl From<&Val> for Value {
    fn from(v: &Val) -> Self {
        match v {
            Val::Null => Value::Null,
            Val::Bool(b) => Value::Bool(*b),
            Val::Int(i) => json!(i),
            Val::Float(f) => json!(f),
            Val::Str(s) => Value::String(s.clone()),
        }
    }
}

/// A recorded SQL query profile entry (matching `padi_core::database::QueryLog`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct QueryLog {
    pub sql: String,
    pub bindings: Vec<Value>,
    pub duration: String,
}

tokio::task_local! {
    pub static REQUEST_QUERIES: std::sync::Arc<std::sync::Mutex<Vec<QueryLog>>>;
}

/// Max profiled queries kept per request (bounds memory for large batch/transaction calls).
const MAX_PROFILED_QUERIES: usize = 200;
/// Max characters of a single binding kept in the profiler output.
const MAX_PROFILED_BINDING_CHARS: usize = 200;

/// Mask secrets (password hashes, JWTs) and truncate huge values before they enter the
/// debug profiler, which is serialized into API responses.
fn redact_binding(v: Value) -> Value {
    match v {
        Value::String(s) => {
            if s.starts_with("$argon2") || s.starts_with("$2") || s.starts_with("eyJ") {
                Value::String("[redacted]".to_string())
            } else if s.chars().count() > MAX_PROFILED_BINDING_CHARS {
                let cut: String = s.chars().take(MAX_PROFILED_BINDING_CHARS).collect();
                Value::String(format!("{}… (truncated)", cut))
            } else {
                Value::String(s)
            }
        }
        other => other,
    }
}

fn clean_sql_for_log(sql: &str) -> String {
    let mut result = String::with_capacity(sql.len());
    let mut chars = sql.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '-' && chars.peek() == Some(&'-') {
            chars.next();
            while let Some(&nc) = chars.peek() {
                chars.next();
                if nc == '\n' {
                    result.push(' ');
                    break;
                }
            }
        } else if c == '/' && chars.peek() == Some(&'*') {
            chars.next();
            while let Some(nc) = chars.next() {
                if nc == '*' && chars.peek() == Some(&'/') {
                    chars.next();
                    break;
                }
            }
        } else {
            result.push(c);
        }
    }
    let collapsed = result.split_whitespace().collect::<Vec<_>>().join(" ");
    if collapsed.is_empty() {
        sql.split_whitespace().collect::<Vec<_>>().join(" ")
    } else {
        collapsed
    }
}

pub fn record_query(sql: &str, params: &[Val], start: std::time::Instant) {
    // Profiling is only active inside a request scope with debug enabled; skip all work otherwise.
    let _ = REQUEST_QUERIES.try_with(|cell| {
        let duration = start.elapsed();
        let ms = (duration.as_secs_f64() * 1000.0 * 100.0).round() / 100.0;
        if let Ok(mut logs) = cell.lock() {
            if logs.len() >= MAX_PROFILED_QUERIES {
                return;
            }
            logs.push(QueryLog {
                sql: clean_sql_for_log(sql),
                bindings: params
                    .iter()
                    .map(|p| redact_binding(Value::from(p)))
                    .collect(),
                duration: format!("{:.2}ms", ms),
            });
        }
    });
}

pub fn current_queries() -> Vec<QueryLog> {
    REQUEST_QUERIES
        .try_with(|cell| cell.lock().map(|logs| logs.clone()).unwrap_or_default())
        .unwrap_or_default()
}

pub fn current_query_count() -> u64 {
    REQUEST_QUERIES
        .try_with(|cell| cell.lock().map(|logs| logs.len() as u64).unwrap_or(0))
        .unwrap_or(0)
}

/// Case-insensitive JSON object property lookup (handles Postgres lowercase column folding & MySQL uppercase).
pub fn json_get<'a>(row: &'a Value, key: &str) -> Option<&'a Value> {
    if let Some(v) = row.get(key) {
        return Some(v);
    }
    if let Some(obj) = row.as_object() {
        for (k, v) in obj {
            if k.eq_ignore_ascii_case(key) {
                return Some(v);
            }
        }
    }
    None
}

pub fn json_get_str(row: &Value, key: &str) -> String {
    json_get(row, key)
        .and_then(|v| match v {
            Value::String(s) => Some(s.clone()),
            Value::Null => None,
            other => Some(other.to_string()),
        })
        .unwrap_or_default()
}

pub fn json_get_opt_str(row: &Value, key: &str) -> Option<String> {
    json_get(row, key).and_then(|v| match v {
        Value::String(s) => Some(s.clone()),
        Value::Null => None,
        other => Some(other.to_string()),
    })
}

pub fn json_get_i64(row: &Value, key: &str) -> i64 {
    json_get(row, key)
        .and_then(|v| {
            v.as_i64()
                .or_else(|| v.as_u64().map(|u| u as i64))
                .or_else(|| v.as_str().and_then(|s| s.parse::<i64>().ok()))
        })
        .unwrap_or(0)
}

impl DbPool {
    pub fn new_sqlite(p: sqlx::SqlitePool) -> Self {
        Self {
            driver_pool: DbDriverPool::Sqlite(p),
            cache: Arc::new(SchemaCache::default()),
        }
    }

    pub fn new_mysql(p: sqlx::MySqlPool, driver: DbDriver) -> Self {
        Self {
            driver_pool: DbDriverPool::MySql(p, driver),
            cache: Arc::new(SchemaCache::default()),
        }
    }

    pub fn new_postgres(p: sqlx::PgPool) -> Self {
        Self {
            driver_pool: DbDriverPool::Postgres(p),
            cache: Arc::new(SchemaCache::default()),
        }
    }

    #[inline]
    pub fn invalidate_schema_cache(&self, table_or_col_name: &str) {
        if let Ok(mut cache) = self.cache.table_metadata.write() {
            cache.remove(table_or_col_name);
        }
        if let Ok(mut cache) = self.cache.relations.write() {
            cache.remove(table_or_col_name);
        }
        if let Ok(mut cache) = self.cache.collections.write() {
            cache.remove(table_or_col_name);
        }
        if let Ok(mut list_cache) = self.cache.collections_list.write() {
            *list_cache = None;
        }
    }

    #[inline]
    pub fn clear_all_schema_cache(&self) {
        if let Ok(mut cache) = self.cache.table_metadata.write() {
            cache.clear();
        }
        if let Ok(mut cache) = self.cache.relations.write() {
            cache.clear();
        }
        if let Ok(mut cache) = self.cache.collections.write() {
            cache.clear();
        }
        if let Ok(mut list_cache) = self.cache.collections_list.write() {
            *list_cache = None;
        }
        if let Ok(mut cache) = self.cache.endpoints.write() {
            cache.clear();
        }
        if let Ok(mut cache) = self.cache.settings.write() {
            *cache = None;
        }
    }

    #[inline]
    pub fn get_cached_collection(&self, name_or_id: &str) -> Option<Collection> {
        let cache = self.cache.collections.read().ok()?;
        cache.get(name_or_id).cloned()
    }

    #[inline]
    pub fn set_cached_collection(&self, col: &Collection) {
        if let Ok(mut cache) = self.cache.collections.write() {
            cache.insert(col.id.clone(), col.clone());
            cache.insert(col.name.clone(), col.clone());
        }
        if let Ok(mut list_cache) = self.cache.collections_list.write() {
            *list_cache = None;
        }
    }

    #[inline]
    pub fn get_cached_metadata(&self, table_name: &str) -> Option<TableMetadata> {
        let cache = self.cache.table_metadata.read().ok()?;
        cache.get(table_name).cloned()
    }

    #[inline]
    pub fn set_cached_metadata(&self, table_name: &str, meta: TableMetadata) {
        if let Ok(mut cache) = self.cache.table_metadata.write() {
            cache.insert(table_name.to_string(), meta);
        }
    }

    #[inline]
    pub fn get_cached_relations(&self, table_name: &str) -> Option<Vec<DetectedRelation>> {
        let cache = self.cache.relations.read().ok()?;
        cache.get(table_name).cloned()
    }

    #[inline]
    pub fn set_cached_relations(&self, table_name: &str, rels: Vec<DetectedRelation>) {
        if let Ok(mut cache) = self.cache.relations.write() {
            cache.insert(table_name.to_string(), rels);
        }
    }

    #[inline]
    pub fn get_cached_settings(&self) -> Option<serde_json::Value> {
        let cache = self.cache.settings.read().ok()?;
        cache.clone()
    }

    #[inline]
    pub fn set_cached_settings(&self, settings: serde_json::Value) {
        if let Ok(mut cache) = self.cache.settings.write() {
            *cache = Some(settings);
        }
    }

    pub fn driver(&self) -> DbDriver {
        match &self.driver_pool {
            DbDriverPool::Sqlite(_) => DbDriver::Sqlite,
            DbDriverPool::MySql(_, d) => *d,
            DbDriverPool::Postgres(_) => DbDriver::Postgres,
        }
    }

    pub fn driver_name(&self) -> &'static str {
        self.driver().as_str()
    }

    pub fn is_sqlite(&self) -> bool {
        matches!(&self.driver_pool, DbDriverPool::Sqlite(_))
    }

    pub fn is_mysql(&self) -> bool {
        matches!(&self.driver_pool, DbDriverPool::MySql(_, _))
    }

    pub fn is_postgres(&self) -> bool {
        matches!(&self.driver_pool, DbDriverPool::Postgres(_))
    }

    /// Translate SQL dialect details:
    /// - For PostgreSQL: converts backtick identifiers `` `col` `` to `"col"` and `?` placeholders to `$1, $2, ...`.
    /// - For SQLite and MySQL/MariaDB: preserves `` `col` `` and `?`.
    pub fn prepare_sql(&self, sql: &str) -> String {
        if !self.is_postgres() {
            return sql.to_string();
        }

        let mut out = String::with_capacity(sql.len() + 16);
        let mut param_idx = 0usize;
        let mut in_single_quote = false;
        let mut in_backtick = false;

        let mut chars = sql.chars().peekable();
        while let Some(c) = chars.next() {
            if c == '\'' && !in_backtick {
                out.push(c);
                if in_single_quote {
                    if chars.peek() == Some(&'\'') {
                        if let Some(escaped) = chars.next() {
                            out.push(escaped);
                        }
                    } else {
                        in_single_quote = false;
                    }
                } else {
                    in_single_quote = true;
                }
            } else if c == '`' && !in_single_quote {
                in_backtick = !in_backtick;
                out.push('"');
            } else if c == '?' && !in_single_quote && !in_backtick {
                param_idx += 1;
                out.push('$');
                out.push_str(&param_idx.to_string());
            } else {
                out.push(c);
            }
        }

        out
    }

    /// Execute a query and return all matching rows as JSON objects.
    pub async fn fetch_all(&self, sql: &str, params: &[Val]) -> Result<Vec<Value>> {
        let prepared = self.prepare_sql(sql);
        let start = std::time::Instant::now();
        let res = match &self.driver_pool {
            DbDriverPool::Sqlite(p) => {
                let rows = bind_sqlite(&prepared, params).fetch_all(p).await?;
                rows.iter().map(sqlite_row_to_json).collect()
            }
            DbDriverPool::MySql(p, _) => {
                let rows = bind_mysql(&prepared, params).fetch_all(p).await?;
                rows.iter().map(mysql_row_to_json).collect()
            }
            DbDriverPool::Postgres(p) => {
                let rows = bind_postgres(&prepared, params).fetch_all(p).await?;
                rows.iter().map(pg_row_to_json).collect()
            }
        };
        record_query(&prepared, params, start);
        Ok(res)
    }

    /// Execute a query and return the first matching row if any.
    pub async fn fetch_optional(&self, sql: &str, params: &[Val]) -> Result<Option<Value>> {
        Ok(self.fetch_all(sql, params).await?.into_iter().next())
    }

    /// Execute a query and return the first matching row, or error if none.
    pub async fn fetch_one(&self, sql: &str, params: &[Val]) -> Result<Value> {
        self.fetch_optional(sql, params)
            .await?
            .ok_or_else(|| anyhow!("Expected at least one row from query"))
    }

    /// Execute a statement and return `(rows_affected, last_insert_id)`.
    pub async fn execute(&self, sql: &str, params: &[Val]) -> Result<(u64, Option<i64>)> {
        let prepared = self.prepare_sql(sql);
        let start = std::time::Instant::now();
        let res = match &self.driver_pool {
            DbDriverPool::Sqlite(p) => {
                let r = bind_sqlite(&prepared, params).execute(p).await?;
                (r.rows_affected(), Some(r.last_insert_rowid()))
            }
            DbDriverPool::MySql(p, _) => {
                let r = bind_mysql(&prepared, params).execute(p).await?;
                (r.rows_affected(), Some(r.last_insert_id() as i64))
            }
            DbDriverPool::Postgres(p) => {
                let r = bind_postgres(&prepared, params).execute(p).await?;
                (r.rows_affected(), None)
            }
        };
        record_query(&prepared, params, start);
        Ok(res)
    }

    /// Start a new database transaction (`BEGIN`).
    /// If the returned `DbTx` is dropped without calling `.commit().await`, it automatically rolls back.
    pub async fn begin(&self) -> Result<DbTx<'_>> {
        let start = std::time::Instant::now();
        let tx = match &self.driver_pool {
            DbDriverPool::Sqlite(p) => DbTx::Sqlite(p.begin().await?),
            DbDriverPool::MySql(p, d) => DbTx::MySql(p.begin().await?, *d),
            DbDriverPool::Postgres(p) => DbTx::Postgres(p.begin().await?),
        };
        record_query("BEGIN", &[], start);
        Ok(tx)
    }
}

/// Unified multi-database transaction handle supporting SQLite, MySQL/MariaDB, and PostgreSQL.
pub enum DbTx<'a> {
    Sqlite(sqlx::Transaction<'a, sqlx::Sqlite>),
    MySql(sqlx::Transaction<'a, sqlx::MySql>, DbDriver),
    Postgres(sqlx::Transaction<'a, sqlx::Postgres>),
}

impl<'a> DbTx<'a> {
    pub fn is_postgres(&self) -> bool {
        matches!(self, DbTx::Postgres(_))
    }

    pub fn is_mysql(&self) -> bool {
        matches!(self, DbTx::MySql(_, _))
    }

    pub fn is_sqlite(&self) -> bool {
        matches!(self, DbTx::Sqlite(_))
    }

    /// Execute a query inside the transaction and return all matching rows as JSON objects.
    pub async fn fetch_all(&mut self, pool: &DbPool, sql: &str, params: &[Val]) -> Result<Vec<Value>> {
        let prepared = pool.prepare_sql(sql);
        let start = std::time::Instant::now();
        let res = match self {
            DbTx::Sqlite(tx) => {
                let rows = bind_sqlite(&prepared, params).fetch_all(&mut **tx).await?;
                rows.iter().map(sqlite_row_to_json).collect()
            }
            DbTx::MySql(tx, _) => {
                let rows = bind_mysql(&prepared, params).fetch_all(&mut **tx).await?;
                rows.iter().map(mysql_row_to_json).collect()
            }
            DbTx::Postgres(tx) => {
                let rows = bind_postgres(&prepared, params).fetch_all(&mut **tx).await?;
                rows.iter().map(pg_row_to_json).collect()
            }
        };
        record_query(&prepared, params, start);
        Ok(res)
    }

    /// Execute a query inside the transaction and return the first matching row if any.
    pub async fn fetch_optional(&mut self, pool: &DbPool, sql: &str, params: &[Val]) -> Result<Option<Value>> {
        Ok(self.fetch_all(pool, sql, params).await?.into_iter().next())
    }

    /// Execute a query inside the transaction and return the first matching row, or error if none.
    pub async fn fetch_one(&mut self, pool: &DbPool, sql: &str, params: &[Val]) -> Result<Value> {
        self.fetch_optional(pool, sql, params)
            .await?
            .ok_or_else(|| anyhow!("Expected at least one row from transaction query"))
    }

    /// Execute a statement inside the transaction and return `(rows_affected, last_insert_id)`.
    pub async fn execute(&mut self, pool: &DbPool, sql: &str, params: &[Val]) -> Result<(u64, Option<i64>)> {
        let prepared = pool.prepare_sql(sql);
        let start = std::time::Instant::now();
        let res = match self {
            DbTx::Sqlite(tx) => {
                let r = bind_sqlite(&prepared, params).execute(&mut **tx).await?;
                (r.rows_affected(), Some(r.last_insert_rowid()))
            }
            DbTx::MySql(tx, _) => {
                let r = bind_mysql(&prepared, params).execute(&mut **tx).await?;
                (r.rows_affected(), Some(r.last_insert_id() as i64))
            }
            DbTx::Postgres(tx) => {
                let r = bind_postgres(&prepared, params).execute(&mut **tx).await?;
                (r.rows_affected(), None)
            }
        };
        record_query(&prepared, params, start);
        Ok(res)
    }

    /// Commit the transaction (`COMMIT`).
    pub async fn commit(self) -> Result<()> {
        let start = std::time::Instant::now();
        match self {
            DbTx::Sqlite(tx) => tx.commit().await?,
            DbTx::MySql(tx, _) => tx.commit().await?,
            DbTx::Postgres(tx) => tx.commit().await?,
        }
        record_query("COMMIT", &[], start);
        Ok(())
    }

    /// Rollback the transaction (`ROLLBACK`).
    pub async fn rollback(self) -> Result<()> {
        let start = std::time::Instant::now();
        match self {
            DbTx::Sqlite(tx) => tx.rollback().await?,
            DbTx::MySql(tx, _) => tx.rollback().await?,
            DbTx::Postgres(tx) => tx.rollback().await?,
        }
        record_query("ROLLBACK", &[], start);
        Ok(())
    }
}

fn bind_sqlite<'q>(
    sql: &'q str,
    params: &'q [Val],
) -> sqlx::query::Query<'q, sqlx::Sqlite, sqlx::sqlite::SqliteArguments<'q>> {
    let mut q = sqlx::query(sql);
    for p in params {
        q = match p {
            Val::Null => q.bind(Option::<String>::None),
            Val::Bool(b) => q.bind(if *b { 1i64 } else { 0i64 }),
            Val::Int(i) => q.bind(*i),
            Val::Float(f) => q.bind(*f),
            Val::Str(s) => q.bind(s.as_str()),
        };
    }
    q
}

fn bind_mysql<'q>(
    sql: &'q str,
    params: &'q [Val],
) -> sqlx::query::Query<'q, sqlx::MySql, sqlx::mysql::MySqlArguments> {
    let mut q = sqlx::query(sql);
    for p in params {
        q = match p {
            Val::Null => q.bind(Option::<String>::None),
            Val::Bool(b) => q.bind(if *b { 1i64 } else { 0i64 }),
            Val::Int(i) => q.bind(*i),
            Val::Float(f) => q.bind(*f),
            Val::Str(s) => q.bind(s.as_str()),
        };
    }
    q
}

fn bind_postgres<'q>(
    sql: &'q str,
    params: &'q [Val],
) -> sqlx::query::Query<'q, sqlx::Postgres, sqlx::postgres::PgArguments> {
    let mut q = sqlx::query(sql);
    for p in params {
        q = match p {
            Val::Null => q.bind(Option::<String>::None),
            Val::Bool(b) => q.bind(if *b { 1i64 } else { 0i64 }),
            Val::Int(i) => q.bind(*i),
            Val::Float(f) => q.bind(*f),
            Val::Str(s) => q.bind(s.as_str()),
        };
    }
    q
}

fn parse_json_str_or_val(s: String) -> Value {
    let trimmed = s.trim();
    if (trimmed.starts_with('{') && trimmed.ends_with('}'))
        || (trimmed.starts_with('[') && trimmed.ends_with(']'))
    {
        if let Ok(parsed) = serde_json::from_str::<Value>(trimmed) {
            return parsed;
        }
    }
    Value::String(s)
}

fn sqlite_row_to_json(row: &sqlx::sqlite::SqliteRow) -> Value {
    let mut map = Map::new();
    for (i, col) in row.columns().iter().enumerate() {
        let v = if let Ok(v) = row.try_get::<Option<i64>, _>(i) {
            v.map(|x| json!(x))
        } else if let Ok(v) = row.try_get::<Option<f64>, _>(i) {
            v.map(|x| json!(x))
        } else if let Ok(v) = row.try_get::<Option<bool>, _>(i) {
            v.map(|x| json!(x))
        } else if let Ok(v) = row.try_get::<Option<String>, _>(i) {
            v.map(parse_json_str_or_val)
        } else if let Ok(v) = row.try_get::<Option<Vec<u8>>, _>(i) {
            v.map(|x| String::from_utf8(x).map(parse_json_str_or_val).unwrap_or(Value::Null))
        } else {
            None
        };
        map.insert(col.name().to_string(), v.unwrap_or(Value::Null));
    }
    Value::Object(map)
}

fn mysql_row_to_json(row: &sqlx::mysql::MySqlRow) -> Value {
    let mut map = Map::new();
    for (i, col) in row.columns().iter().enumerate() {
        let v = if let Ok(v) = row.try_get::<Option<i64>, _>(i) {
            v.map(|x| json!(x))
        } else if let Ok(v) = row.try_get::<Option<u64>, _>(i) {
            v.map(|x| json!(x))
        } else if let Ok(v) = row.try_get::<Option<f64>, _>(i) {
            v.map(|x| json!(x))
        } else if let Ok(v) = row.try_get::<Option<bool>, _>(i) {
            v.map(|x| json!(x))
        } else if let Ok(v) = row.try_get::<Option<rust_decimal::Decimal>, _>(i) {
            v.map(|x| json!(x.to_string()))
        } else if let Ok(v) = row.try_get::<Option<chrono::NaiveDateTime>, _>(i) {
            v.map(|x| json!(x.format("%Y-%m-%d %H:%M:%S").to_string()))
        } else if let Ok(v) = row.try_get::<Option<chrono::DateTime<chrono::Utc>>, _>(i) {
            v.map(|x| json!(x.to_rfc3339()))
        } else if let Ok(v) = row.try_get::<Option<chrono::NaiveDate>, _>(i) {
            v.map(|x| json!(x.to_string()))
        } else if let Ok(v) = row.try_get::<Option<chrono::NaiveTime>, _>(i) {
            v.map(|x| json!(x.to_string()))
        } else if let Ok(v) = row.try_get::<Option<String>, _>(i) {
            v.map(parse_json_str_or_val)
        } else if let Ok(v) = row.try_get::<Option<Vec<u8>>, _>(i) {
            v.map(|x| String::from_utf8(x).map(parse_json_str_or_val).unwrap_or(Value::Null))
        } else {
            None
        };
        map.insert(col.name().to_string(), v.unwrap_or(Value::Null));
    }
    Value::Object(map)
}

fn pg_row_to_json(row: &sqlx::postgres::PgRow) -> Value {
    let mut map = Map::new();
    for (i, col) in row.columns().iter().enumerate() {
        let v = if let Ok(v) = row.try_get::<Option<i64>, _>(i) {
            v.map(|x| json!(x))
        } else if let Ok(v) = row.try_get::<Option<i32>, _>(i) {
            v.map(|x| json!(x as i64))
        } else if let Ok(v) = row.try_get::<Option<f64>, _>(i) {
            v.map(|x| json!(x))
        } else if let Ok(v) = row.try_get::<Option<bool>, _>(i) {
            v.map(|x| json!(x))
        } else if let Ok(v) = row.try_get::<Option<rust_decimal::Decimal>, _>(i) {
            v.map(|x| json!(x.to_string()))
        } else if let Ok(v) = row.try_get::<Option<chrono::DateTime<chrono::Utc>>, _>(i) {
            v.map(|x| json!(x.to_rfc3339()))
        } else if let Ok(v) = row.try_get::<Option<chrono::NaiveDateTime>, _>(i) {
            v.map(|x| json!(x.format("%Y-%m-%d %H:%M:%S").to_string()))
        } else if let Ok(v) = row.try_get::<Option<chrono::NaiveDate>, _>(i) {
            v.map(|x| json!(x.to_string()))
        } else if let Ok(v) = row.try_get::<Option<chrono::NaiveTime>, _>(i) {
            v.map(|x| json!(x.to_string()))
        } else if let Ok(v) = row.try_get::<Option<String>, _>(i) {
            v.map(parse_json_str_or_val)
        } else if let Ok(v) = row.try_get::<Option<Vec<u8>>, _>(i) {
            v.map(|x| String::from_utf8(x).map(parse_json_str_or_val).unwrap_or(Value::Null))
        } else {
            None
        };
        map.insert(col.name().to_string(), v.unwrap_or(Value::Null));
    }
    Value::Object(map)
}

/// Resolve database URL and driver from explicit override, `.env` / environment variables, or fallback to `<data_dir>/data.db`.
pub fn resolve_db_config(data_dir: &Path, db_url_override: Option<&str>) -> (String, DbDriver) {
    Env::load(".env");

    let raw_url = if let Some(u) = db_url_override.filter(|s| !s.trim().is_empty()) {
        u.trim().to_string()
    } else if let Some(u) = Env::get("DATABASE_URL") {
        u
    } else {
        let driver_env = Env::get("DB_DRIVER")
            .or_else(|| Env::get("DB_CONNECTION"))
            .unwrap_or_else(|| {
                // Auto-infer if DB_DRIVER was omitted but network DB host/port is provided
                if let Some(port) = Env::get("DB_PORT") {
                    if port == "3306" {
                        return "mysql".to_string();
                    } else if port == "5432" {
                        return "postgres".to_string();
                    }
                }
                if Env::get("DB_HOST").is_some() && Env::get("DB_DATABASE").is_some() {
                    return "mysql".to_string();
                }
                "sqlite".to_string()
            })
            .to_lowercase();

        if driver_env == "sqlite" {
            let db_path = if data_dir == Path::new("./padi_data") {
                Env::get("DB_DATABASE")
                    .map(std::path::PathBuf::from)
                    .unwrap_or_else(|| data_dir.join("data.db"))
            } else {
                data_dir.join("data.db")
            };
            format!("sqlite://{}", db_path.to_string_lossy())
        } else {
            let is_pg = driver_env == "pgsql" || driver_env == "postgres" || driver_env == "postgresql";
            let is_mariadb = driver_env == "mariadb";
            let scheme = if is_pg {
                "postgres"
            } else if is_mariadb {
                "mariadb"
            } else {
                "mysql"
            };
            let user = Env::get_or("DB_USERNAME", if is_pg { "postgres" } else { "root" });
            let pass = Env::get_or("DB_PASSWORD", "");
            let host = Env::get_or("DB_HOST", "127.0.0.1");
            let port = Env::get_or("DB_PORT", if is_pg { "5432" } else { "3306" });
            let db_name = Env::get_or("DB_DATABASE", "db_padi");

            if pass.is_empty() {
                format!("{}://{}@{}:{}/{}", scheme, user, host, port, db_name)
            } else {
                format!("{}://{}:{}@{}:{}/{}", scheme, user, pass, host, port, db_name)
            }
        }
    };

    if raw_url.starts_with("postgres://") || raw_url.starts_with("postgresql://") {
        (raw_url, DbDriver::Postgres)
    } else if let Some(rest) = raw_url.strip_prefix("mariadb://") {
        (format!("mysql://{}", rest), DbDriver::MariaDb)
    } else if raw_url.starts_with("mysql://") {
        let drv = if Env::get("DB_DRIVER")
            .or_else(|| Env::get("DB_CONNECTION"))
            .map(|d| d.eq_ignore_ascii_case("mariadb"))
            .unwrap_or(false)
        {
            DbDriver::MariaDb
        } else {
            DbDriver::MySql
        };
        (raw_url, drv)
    } else if raw_url.starts_with("sqlite:") {
        (raw_url, DbDriver::Sqlite)
    } else {
        (format!("sqlite://{}", raw_url), DbDriver::Sqlite)
    }
}

pub async fn init_db<P: AsRef<Path>>(data_dir: P) -> Result<DbPool> {
    init_db_with_url(data_dir, None).await
}

pub async fn init_db_with_url<P: AsRef<Path>>(
    data_dir: P,
    db_url_override: Option<&str>,
) -> Result<DbPool> {
    clear_all_schema_cache();
    let dir = data_dir.as_ref();
    if !dir.exists() {
        std::fs::create_dir_all(dir)?;
    }

    let (db_url, driver) = resolve_db_config(dir, db_url_override);
    let max_conn = Env::get_int("DB_MAX_CONNECTIONS", 20).max(1) as u32;

    let pool = match driver {
        DbDriver::Sqlite => {
            if let Some(path_part) = db_url.strip_prefix("sqlite://") {
                let clean_path = path_part.split('?').next().unwrap_or(path_part);
                if clean_path != ":memory:" {
                    if let Some(parent) = Path::new(clean_path).parent() {
                        if !parent.as_os_str().is_empty() && !parent.exists() {
                            let _ = std::fs::create_dir_all(parent);
                        }
                    }
                }
            }
            let options = SqliteConnectOptions::from_str(&db_url)?
                .create_if_missing(true)
                .journal_mode(SqliteJournalMode::Wal)
                .synchronous(SqliteSynchronous::Normal)
                .busy_timeout(std::time::Duration::from_secs(15));

            let p = SqlitePoolOptions::new()
                .max_connections(max_conn)
                .connect_with(options)
                .await?;
            info!("Connected to SQLite database ({})", db_url);
            DbPool::new_sqlite(p)
        }
        DbDriver::MySql | DbDriver::MariaDb => {
            let p = MySqlPoolOptions::new()
                .max_connections(max_conn)
                .connect(&db_url)
                .await?;
            info!("Connected to {} database", driver.as_str().to_uppercase());
            DbPool::new_mysql(p, driver)
        }
        DbDriver::Postgres => {
            let p = PgPoolOptions::new()
                .max_connections(max_conn)
                .connect(&db_url)
                .await?;
            info!("Connected to PostgreSQL database");
            DbPool::new_postgres(p)
        }
    };

    bootstrap_system_tables(&pool).await?;

    Ok(pool)
}

async fn bootstrap_system_tables(pool: &DbPool) -> Result<()> {
    // 1. _collections table (VARCHAR(255) for PK/UNIQUE compatibility across MySQL/MariaDB, Postgres, SQLite)
    pool.execute(
        r#"
        CREATE TABLE IF NOT EXISTS `_collections` (
            `id` VARCHAR(255) PRIMARY KEY,
            `name` VARCHAR(255) UNIQUE NOT NULL,
            `type` VARCHAR(64) NOT NULL,
            `schema` TEXT NOT NULL,
            `realtime` INTEGER DEFAULT 1,
            `autoLoadRelations` INTEGER DEFAULT 0,
            `blameable` INTEGER DEFAULT 1,
            `listRule` TEXT,
            `allRule` TEXT,
            `viewRule` TEXT,
            `createRule` TEXT,
            `updateRule` TEXT,
            `deleteRule` TEXT,
            `options` TEXT,
            `created` VARCHAR(64) NOT NULL,
            `updated` VARCHAR(64) NOT NULL
        );
        "#,
        &[],
    )
    .await?;

    // Ensure `realtime`, `autoLoadRelations`, and `blameable` columns exist on existing `_collections` tables
    if let Ok(cols) = get_existing_table_columns(pool, "_collections").await {
        if !cols.iter().any(|c| c.eq_ignore_ascii_case("realtime")) {
            let _ = pool
                .execute(
                    "ALTER TABLE `_collections` ADD COLUMN `realtime` INTEGER DEFAULT 1;",
                    &[],
                )
                .await;
        }
        if !cols.iter().any(|c| c.eq_ignore_ascii_case("autoLoadRelations") || c.eq_ignore_ascii_case("auto_load_relations")) {
            let _ = pool
                .execute(
                    "ALTER TABLE `_collections` ADD COLUMN `autoLoadRelations` INTEGER DEFAULT 0;",
                    &[],
                )
                .await;
        }
        if !cols.iter().any(|c| c.eq_ignore_ascii_case("blameable")) {
            let _ = pool
                .execute(
                    "ALTER TABLE `_collections` ADD COLUMN `blameable` INTEGER DEFAULT 1;",
                    &[],
                )
                .await;
        }
        if !cols.iter().any(|c| c.eq_ignore_ascii_case("allRule") || c.eq_ignore_ascii_case("all_rule")) {
            let _ = pool
                .execute(
                    "ALTER TABLE `_collections` ADD COLUMN `allRule` TEXT;",
                    &[],
                )
                .await;
        }
        if !cols.iter().any(|c| c.eq_ignore_ascii_case("options")) {
            let _ = pool
                .execute(
                    "ALTER TABLE `_collections` ADD COLUMN `options` TEXT;",
                    &[],
                )
                .await;
        }
    }

    // 2. _admins table
    pool.execute(
        r#"
        CREATE TABLE IF NOT EXISTS `_admins` (
            `id` VARCHAR(255) PRIMARY KEY,
            `email` VARCHAR(255) UNIQUE NOT NULL,
            `passwordHash` TEXT NOT NULL,
            `tokenKey` VARCHAR(255) NOT NULL,
            `created` VARCHAR(64) NOT NULL,
            `updated` VARCHAR(64) NOT NULL
        );
        "#,
        &[],
    )
    .await?;

    // 3. _params table (key-value settings; `key` quoted for MySQL/MariaDB compatibility)
    pool.execute(
        r#"
        CREATE TABLE IF NOT EXISTS `_params` (
            `key` VARCHAR(255) PRIMARY KEY,
            `value` TEXT NOT NULL
        );
        "#,
        &[],
    )
    .await?;

    // 3b. _endpoints table (Dynamic Custom SQL Endpoints)
    pool.execute(
        r#"
        CREATE TABLE IF NOT EXISTS `_endpoints` (
            `id` VARCHAR(255) PRIMARY KEY,
            `name` VARCHAR(255) NOT NULL,
            `path` VARCHAR(255) UNIQUE NOT NULL,
            `method` VARCHAR(32) NOT NULL,
            `sql` TEXT NOT NULL,
            `description` TEXT,
            `authRule` TEXT,
            `transaction` INTEGER DEFAULT 0,
            `singleRow` INTEGER DEFAULT 0,
            `created` VARCHAR(64) NOT NULL,
            `updated` VARCHAR(64) NOT NULL
        );
        "#,
        &[],
    )
    .await?;

    // Preload system settings and custom endpoints into RAM cache once at startup
    let _ = get_system_settings(pool).await;
    let _ = get_all_custom_endpoints(pool).await;

    // 4. Default 'users' collection if not exists
    let existing_users = pool
        .fetch_optional(
            "SELECT `id` FROM `_collections` WHERE `name` = ?",
            &[Val::from("users")],
        )
        .await?;

    if existing_users.is_none() {
        let default_users_collection = Collection {
            id: "system_users".to_string(),
            name: "users".to_string(),
            collection_type: CollectionType::Auth,
            schema: vec![
                super::schema::SchemaField {
                    name: "name".to_string(),
                    field_type: FieldType::Text,
                    required: false,
                    unique: false,
                    options: serde_json::json!({}),
                },
                super::schema::SchemaField {
                    name: "role".to_string(),
                    field_type: FieldType::Text,
                    required: false,
                    unique: false,
                    options: serde_json::json!({ "default": "user" }),
                },
                super::schema::SchemaField {
                    name: "status".to_string(),
                    field_type: FieldType::Text,
                    required: false,
                    unique: false,
                    options: serde_json::json!({ "default": "active" }),
                },
                super::schema::SchemaField {
                    name: "email_verified_at".to_string(),
                    field_type: FieldType::Number,
                    required: false,
                    unique: false,
                    options: serde_json::json!({}),
                },
                super::schema::SchemaField {
                    name: "remember_token".to_string(),
                    field_type: FieldType::Text,
                    required: false,
                    unique: false,
                    options: serde_json::json!({}),
                },
                super::schema::SchemaField {
                    name: "last_login_at".to_string(),
                    field_type: FieldType::Text,
                    required: false,
                    unique: false,
                    options: serde_json::json!({}),
                },
                super::schema::SchemaField {
                    name: "created_by".to_string(),
                    field_type: FieldType::Text,
                    required: false,
                    unique: false,
                    options: serde_json::json!({}),
                },
                super::schema::SchemaField {
                    name: "updated_by".to_string(),
                    field_type: FieldType::Text,
                    required: false,
                    unique: false,
                    options: serde_json::json!({}),
                },
                super::schema::SchemaField {
                    name: "avatar".to_string(),
                    field_type: FieldType::File,
                    required: false,
                    unique: false,
                    options: serde_json::json!({ "maxSelect": 1 }),
                },
            ],
            realtime: true,
            auto_load_relations: false,
            blameable: true,
            list_rule: Some("".to_string()),
            all_rule: Some("".to_string()),
            view_rule: Some("".to_string()),
            create_rule: Some("".to_string()),
            update_rule: Some("id = @request.auth.id".to_string()),
            delete_rule: Some("id = @request.auth.id".to_string()),
            resource_fields: None,
            resource_relations: None,
            resource_relation_fields: None,
            list_fields: None,
            all_fields: None,
            view_fields: None,
            list_relations: None,
            all_relations: None,
            view_relations: None,
            created: chrono::Utc::now().to_rfc3339(),
            updated: chrono::Utc::now().to_rfc3339(),
        };

        create_or_update_collection_table(pool, &default_users_collection).await?;
        save_collection_metadata(pool, &default_users_collection).await?;
    }

    // 5. Introspect and sync existing physical database tables into _collections
    if let Ok(synced) = sync_existing_tables(pool).await {
        if !synced.is_empty() {
            info!(
                "Auto-synced {} existing physical database table(s) into collections: {:?}",
                synced.len(),
                synced
            );
        }
    }

    Ok(())
}

pub fn default_system_settings() -> Value {
    json!({
        "meta": {
            "appName": Env::get_or("APP_NAME", "PadiBase Studio"),
            "appUrl": format!("http://{}:{}", Env::get_or("APP_HOST", "127.0.0.1"), Env::get_or("APP_PORT", "8085")),
            "hideControls": false
        },
        "roles": [
            "admin",
            "user"
        ],
        "smtp": {
            "enabled": Env::get_or("SMTP_ENABLED", "false").eq_ignore_ascii_case("true") || Env::get_or("SMTP_ENABLED", "false") == "1",
            "host": Env::get_or("SMTP_HOST", ""),
            "port": Env::get_or("SMTP_PORT", "587").parse::<u16>().unwrap_or(587),
            "username": Env::get_or("SMTP_USERNAME", ""),
            "password": Env::get_or("SMTP_PASSWORD", ""),
            "encryption": Env::get_or("SMTP_ENCRYPTION", "opportunistic"),
            "auth_method": Env::get_or("SMTP_AUTH_METHOD", "LOGIN"),
            "ehlo_domain": Env::get_or("SMTP_EHLO_DOMAIN", ""),
            "from_name": Env::get_or("SMTP_FROM_NAME", &Env::get_or("APP_NAME", "PadiBase Studio")),
            "from_address": Env::get_or("SMTP_FROM_ADDRESS", "noreply@example.com"),
            "events": {
                "welcome": true,
                "forgot_password": true,
                "password_changed": true,
                "admin_created": true
            },
            "templates": {
                "welcome": crate::core::email::default_welcome_template(),
                "forgot_password": crate::core::email::default_forgot_password_template(),
                "password_changed": crate::core::email::default_password_changed_template(),
                "admin_created": crate::core::email::default_admin_created_template()
            }
        }
    })
}

/// Retrieve the list of active user roles configured in system settings.
pub async fn get_available_roles(pool: &DbPool) -> Vec<String> {
    if let Ok(settings) = get_system_settings(pool).await {
        if let Some(roles_val) = settings.get("roles").and_then(|v| v.as_array()) {
            let roles: Vec<String> = roles_val
                .iter()
                .filter_map(|r| r.as_str().map(|s| s.trim().to_lowercase()))
                .filter(|s| !s.is_empty())
                .collect();
            if !roles.is_empty() {
                return roles;
            }
        }
    }
    vec![
        "admin".to_string(),
        "user".to_string(),
    ]
}

/// Get system settings from in-memory tenant cache, falling back to database once.
pub async fn get_system_settings(pool: &DbPool) -> Result<Value> {
    if let Some(cached) = pool.get_cached_settings() {
        return Ok(cached);
    }
    let row = pool
        .fetch_optional(
            "SELECT `value` FROM `_params` WHERE `key` = 'settings'",
            &[],
        )
        .await?;

    let val = if let Some(r) = row {
        let val_str = json_get_str(&r, "value");
        serde_json::from_str::<Value>(&val_str).unwrap_or_else(|_| default_system_settings())
    } else {
        default_system_settings()
    };

    pool.set_cached_settings(val.clone());
    Ok(val)
}

/// Save system settings into database and IMMEDIATELY reload into in-memory cache.
pub async fn save_system_settings(pool: &DbPool, new_settings: Value) -> Result<Value> {
    let mut current = get_system_settings(pool).await.unwrap_or_else(|_| default_system_settings());
    if let (Some(curr_obj), Some(new_obj)) = (current.as_object_mut(), new_settings.as_object()) {
        for (k, v) in new_obj {
            if k == "meta" || k == "smtp" {
                if let (Some(curr_sub), Some(new_sub)) = (curr_obj.get_mut(k).and_then(|m| m.as_object_mut()), v.as_object()) {
                    for (sk, sv) in new_sub {
                        if sk == "events" {
                            if let (Some(curr_events), Some(new_events)) = (curr_sub.get_mut("events").and_then(|e| e.as_object_mut()), sv.as_object()) {
                                for (ek, ev) in new_events {
                                    curr_events.insert(ek.clone(), ev.clone());
                                }
                            } else {
                                curr_sub.insert(sk.clone(), sv.clone());
                            }
                        } else if sk == "templates" {
                            if let (Some(curr_tmpls), Some(new_tmpls)) = (curr_sub.get_mut("templates").and_then(|t| t.as_object_mut()), sv.as_object()) {
                                for (tk, tv) in new_tmpls {
                                    curr_tmpls.insert(tk.clone(), tv.clone());
                                }
                            } else {
                                curr_sub.insert(sk.clone(), sv.clone());
                            }
                        } else {
                            curr_sub.insert(sk.clone(), sv.clone());
                        }
                    }
                } else {
                    curr_obj.insert(k.clone(), v.clone());
                }
            } else {
                curr_obj.insert(k.clone(), v.clone());
            }
        }
    } else {
        current = new_settings;
    }

    let val_str = serde_json::to_string(&current).unwrap_or_else(|_| "{}".to_string());
    let sql = if pool.is_mysql() {
        "INSERT INTO `_params` (`key`, `value`) VALUES ('settings', ?) ON DUPLICATE KEY UPDATE `value` = VALUES(`value`)"
    } else {
        "INSERT INTO `_params` (`key`, `value`) VALUES ('settings', ?) ON CONFLICT(`key`) DO UPDATE SET `value` = excluded.`value`"
    };

    pool.execute(sql, &[Val::from(&val_str)]).await?;

    // Reload into memory cache immediately
    pool.set_cached_settings(current.clone());

    // Sync runtime APP_NAME if provided in settings
    if let Some(app_name) = current.get("meta").and_then(|m| m.get("appName")).and_then(|n| n.as_str()) {
        if !app_name.trim().is_empty() {
            Env::set("APP_NAME", app_name);
        }
    }

    Ok(current)
}

pub async fn save_collection_metadata(pool: &DbPool, col: &Collection) -> Result<()> {
    let schema_json = serde_json::to_string(&col.schema)?;
    let col_type = match col.collection_type {
        CollectionType::Base => "base",
        CollectionType::Auth => "auth",
        CollectionType::View => "view",
    };
    let realtime_int = if col.realtime { 1i64 } else { 0i64 };
    let auto_load_int = if col.auto_load_relations { 1i64 } else { 0i64 };
    let blameable_int = if col.blameable { 1i64 } else { 0i64 };

    let options_json = serde_json::json!({
        "resourceFields": col.resource_fields,
        "resourceRelations": col.resource_relations,
        "resourceRelationFields": col.resource_relation_fields,
        "listFields": col.list_fields,
        "allFields": col.all_fields,
        "viewFields": col.view_fields,
        "listRelations": col.list_relations,
        "allRelations": col.all_relations,
        "viewRelations": col.view_relations,
    }).to_string();

    let sql = if pool.is_mysql() {
        r#"
        INSERT INTO `_collections` (`id`, `name`, `type`, `schema`, `realtime`, `autoLoadRelations`, `blameable`, `listRule`, `allRule`, `viewRule`, `createRule`, `updateRule`, `deleteRule`, `options`, `created`, `updated`)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        ON DUPLICATE KEY UPDATE
            `name` = VALUES(`name`),
            `type` = VALUES(`type`),
            `schema` = VALUES(`schema`),
            `realtime` = VALUES(`realtime`),
            `autoLoadRelations` = VALUES(`autoLoadRelations`),
            `blameable` = VALUES(`blameable`),
            `listRule` = VALUES(`listRule`),
            `allRule` = VALUES(`allRule`),
            `viewRule` = VALUES(`viewRule`),
            `createRule` = VALUES(`createRule`),
            `updateRule` = VALUES(`updateRule`),
            `deleteRule` = VALUES(`deleteRule`),
            `options` = VALUES(`options`),
            `updated` = VALUES(`updated`)
        "#
    } else {
        r#"
        INSERT INTO `_collections` (`id`, `name`, `type`, `schema`, `realtime`, `autoLoadRelations`, `blameable`, `listRule`, `allRule`, `viewRule`, `createRule`, `updateRule`, `deleteRule`, `options`, `created`, `updated`)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        ON CONFLICT(`id`) DO UPDATE SET
            `name` = excluded.`name`,
            `type` = excluded.`type`,
            `schema` = excluded.`schema`,
            `realtime` = excluded.`realtime`,
            `autoLoadRelations` = excluded.`autoLoadRelations`,
            `blameable` = excluded.`blameable`,
            `listRule` = excluded.`listRule`,
            `allRule` = excluded.`allRule`,
            `viewRule` = excluded.`viewRule`,
            `createRule` = excluded.`createRule`,
            `updateRule` = excluded.`updateRule`,
            `deleteRule` = excluded.`deleteRule`,
            `options` = excluded.`options`,
            `updated` = excluded.`updated`
        "#
    };

    pool.execute(
        sql,
        &[
            Val::from(&col.id),
            Val::from(&col.name),
            Val::from(col_type),
            Val::from(schema_json),
            Val::Int(realtime_int),
            Val::Int(auto_load_int),
            Val::Int(blameable_int),
            Val::from(col.list_rule.clone()),
            Val::from(col.all_rule.clone()),
            Val::from(col.view_rule.clone()),
            Val::from(col.create_rule.clone()),
            Val::from(col.update_rule.clone()),
            Val::from(col.delete_rule.clone()),
            Val::from(options_json),
            Val::from(&col.created),
            Val::from(&col.updated),
        ],
    )
    .await?;

    // Invalidate and update tenant in-memory cache
    pool.invalidate_schema_cache(&col.name);
    pool.invalidate_schema_cache(&col.id);
    pool.set_cached_collection(col);

    // Immediately pre-warm introspected table metadata & relations into RAM
    let _ = get_table_metadata(pool, &col.name).await;
    let _ = get_table_relations(pool, &col.name).await;

    Ok(())
}

fn value_to_collection(r: &Value) -> Collection {
    let type_str = json_get_str(r, "type");
    let collection_type = match type_str.as_str() {
        "auth" => CollectionType::Auth,
        "view" => CollectionType::View,
        _ => CollectionType::Base,
    };

    let schema_raw = json_get_str(r, "schema");
    let schema = serde_json::from_str(&schema_raw).unwrap_or_default();

    let realtime = match json_get(r, "realtime") {
        Some(Value::Bool(b)) => *b,
        Some(Value::Number(n)) => n.as_i64().unwrap_or(1) != 0,
        Some(Value::String(s)) => s != "0" && !s.eq_ignore_ascii_case("false"),
        _ => true,
    };

    let auto_load_relations = match json_get(r, "autoLoadRelations").or_else(|| json_get(r, "auto_load_relations")) {
        Some(Value::Bool(b)) => *b,
        Some(Value::Number(n)) => n.as_i64().unwrap_or(0) != 0,
        Some(Value::String(s)) => s == "1" || s.eq_ignore_ascii_case("true"),
        _ => false,
    };

    let blameable = match json_get(r, "blameable") {
        Some(Value::Bool(b)) => *b,
        Some(Value::Number(n)) => n.as_i64().unwrap_or(0) != 0,
        Some(Value::String(s)) => s == "1" || s.eq_ignore_ascii_case("true"),
        _ => false,
    };

    let (resource_fields, resource_relations, resource_relation_fields, list_fields, all_fields, view_fields, list_relations, all_relations, view_relations) = {
        let opts_val = json_get(r, "options").and_then(|v| {
            if let Value::String(s) = v {
                serde_json::from_str::<Value>(s).ok()
            } else if v.is_object() {
                Some(v.clone())
            } else {
                None
            }
        });

        let extract_vec = |key: &str| -> Option<Vec<String>> {
            if let Some(ref opts) = opts_val {
                if let Some(arr) = opts.get(key).and_then(|v| v.as_array()) {
                    return Some(arr.iter().filter_map(|x| x.as_str().map(|s| s.to_string())).collect());
                }
            }
            if let Some(arr) = json_get(r, key).and_then(|v| v.as_array()) {
                return Some(arr.iter().filter_map(|x| x.as_str().map(|s| s.to_string())).collect());
            }
            None
        };

        let extract_map = |key: &str| -> Option<std::collections::HashMap<String, Vec<String>>> {
            let parse_obj = |obj: &serde_json::Map<String, serde_json::Value>| -> Option<std::collections::HashMap<String, Vec<String>>> {
                let mut map = std::collections::HashMap::new();
                for (k, v) in obj {
                    if let Some(arr) = v.as_array() {
                        let fields: Vec<String> = arr.iter().filter_map(|x| x.as_str().map(|s| s.to_string())).collect();
                        if !fields.is_empty() {
                            map.insert(k.clone(), fields);
                        }
                    }
                }
                if !map.is_empty() { Some(map) } else { None }
            };

            if let Some(ref opts) = opts_val {
                if let Some(obj) = opts.get(key).and_then(|v| v.as_object()) {
                    if let Some(m) = parse_obj(obj) {
                        return Some(m);
                    }
                }
            }
            if let Some(obj) = json_get(r, key).and_then(|v| v.as_object()) {
                if let Some(m) = parse_obj(obj) {
                    return Some(m);
                }
            }
            None
        };

        (
            extract_vec("resourceFields").or_else(|| extract_vec("resource_fields")),
            extract_vec("resourceRelations").or_else(|| extract_vec("resource_relations")),
            extract_map("resourceRelationFields").or_else(|| extract_map("resource_relation_fields")),
            extract_vec("listFields"),
            extract_vec("allFields"),
            extract_vec("viewFields"),
            extract_vec("listRelations"),
            extract_vec("allRelations"),
            extract_vec("viewRelations"),
        )
    };

    Collection {
        id: json_get_str(r, "id"),
        name: json_get_str(r, "name"),
        collection_type,
        schema,
        realtime,
        auto_load_relations,
        blameable,
        list_rule: json_get_opt_str(r, "listRule"),
        all_rule: json_get_opt_str(r, "allRule"),
        view_rule: json_get_opt_str(r, "viewRule"),
        create_rule: json_get_opt_str(r, "createRule"),
        update_rule: json_get_opt_str(r, "updateRule"),
        delete_rule: json_get_opt_str(r, "deleteRule"),
        resource_fields,
        resource_relations,
        resource_relation_fields,
        list_fields,
        all_fields,
        view_fields,
        list_relations,
        all_relations,
        view_relations,
        created: json_get_str(r, "created"),
        updated: json_get_str(r, "updated"),
    }
}

pub async fn get_collection_by_name_or_id(
    pool: &DbPool,
    name_or_id: &str,
) -> Result<Option<Collection>> {
    // Fast path: Check tenant in-memory cache first
    if let Some(col) = pool.get_cached_collection(name_or_id) {
        return Ok(Some(col));
    }

    let row = pool
        .fetch_optional(
            r#"
            SELECT `id`, `name`, `type`, `schema`, `realtime`, `autoLoadRelations`, `blameable`, `listRule`, `allRule`, `viewRule`, `createRule`, `updateRule`, `deleteRule`, `options`, `created`, `updated`
            FROM `_collections`
            WHERE `id` = ? OR `name` = ?
            "#,
            &[Val::from(name_or_id), Val::from(name_or_id)],
        )
        .await?;

    let col_opt = row.as_ref().map(value_to_collection);
    if let Some(ref col) = col_opt {
        pool.set_cached_collection(col);
    }

    Ok(col_opt)
}

pub async fn list_collections(pool: &DbPool) -> Result<Vec<Collection>> {
    if let Ok(list_guard) = pool.cache.collections_list.read() {
        if let Some(ref cached_list) = *list_guard {
            return Ok(cached_list.clone());
        }
    }

    let rows = pool
        .fetch_all(
            r#"
            SELECT `id`, `name`, `type`, `schema`, `realtime`, `autoLoadRelations`, `blameable`, `listRule`, `allRule`, `viewRule`, `createRule`, `updateRule`, `deleteRule`, `options`, `created`, `updated`
            FROM `_collections`
            ORDER BY `name` ASC
            "#,
            &[],
        )
        .await?;

    let mut list: Vec<Collection> = rows.iter().map(value_to_collection).collect();
    list.sort_by(|a, b| {
        if a.name == "users" {
            std::cmp::Ordering::Less
        } else if b.name == "users" {
            std::cmp::Ordering::Greater
        } else if a.collection_type == crate::core::schema::CollectionType::Auth
            && b.collection_type != crate::core::schema::CollectionType::Auth
        {
            std::cmp::Ordering::Less
        } else if a.collection_type != crate::core::schema::CollectionType::Auth
            && b.collection_type == crate::core::schema::CollectionType::Auth
        {
            std::cmp::Ordering::Greater
        } else {
            a.name.cmp(&b.name)
        }
    });
    // Cache all collections in tenant memory
    if let Ok(mut cache) = pool.cache.collections.write() {
        for col in &list {
            cache.insert(col.id.clone(), col.clone());
            cache.insert(col.name.clone(), col.clone());
        }
    }
    if let Ok(mut list_guard) = pool.cache.collections_list.write() {
        *list_guard = Some(list.clone());
    }
    Ok(list)
}

pub async fn delete_collection(pool: &DbPool, name_or_id: &str) -> Result<bool> {
    let col = match get_collection_by_name_or_id(pool, name_or_id).await? {
        Some(c) => c,
        None => return Ok(false),
    };

    pool.invalidate_schema_cache(&col.name);
    pool.invalidate_schema_cache(&col.id);

    let sql = format!("DROP TABLE IF EXISTS `{}`;", col.name);
    pool.execute(&sql, &[]).await?;

    pool.execute(
        "DELETE FROM `_collections` WHERE `id` = ?",
        &[Val::from(&col.id)],
    )
    .await?;

    Ok(true)
}

/// Rename a collection's physical table and metadata. Relation fields in other
/// collections that point to the old name (`options.targetTable`) are re-pointed.
/// `new_name` must already be validated by the caller.
pub async fn rename_collection(pool: &DbPool, name_or_id: &str, new_name: &str) -> Result<Collection> {
    let mut col = get_collection_by_name_or_id(pool, name_or_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Collection not found"))?;
    let old_name = col.name.clone();
    if old_name == new_name {
        return Ok(col);
    }

    let sql = format!("ALTER TABLE `{}` RENAME TO `{}`;", old_name, new_name);
    pool.execute(&sql, &[]).await?;

    col.name = new_name.to_string();
    col.updated = chrono::Utc::now().to_rfc3339();
    save_collection_metadata(pool, &col).await?;

    // Re-point relation fields in other collections.
    pool.clear_all_schema_cache();
    for mut other in list_collections(pool).await? {
        if other.id == col.id {
            continue;
        }
        let mut changed = false;
        for f in other.schema.iter_mut() {
            if let Some(obj) = f.options.as_object_mut() {
                if obj.get("targetTable").and_then(|v| v.as_str()) == Some(old_name.as_str()) {
                    obj.insert("targetTable".into(), serde_json::Value::String(new_name.to_string()));
                    changed = true;
                }
            }
        }
        if changed {
            save_collection_metadata(pool, &other).await?;
        }
    }

    pool.clear_all_schema_cache();
    Ok(col)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntrospectedColumn {
    pub name: String,
    pub data_type: String,
    pub column_type: String,
    pub is_nullable: bool,
    pub is_unique: bool,
    pub is_pk: bool,
    pub is_auto_increment: bool,
    pub default_value: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableMetadata {
    pub table_name: String,
    pub pk_column: String,
    pub pk_is_int: bool,
    pub pk_is_auto_increment: bool,
    pub has_created: bool,
    pub has_updated: bool,
    pub has_created_at: bool,
    pub has_updated_at: bool,
    pub has_created_by: bool,
    pub has_updated_by: bool,
    pub has_collection_id: bool,
    pub has_collection_name: bool,
    pub columns: Vec<IntrospectedColumn>,
}

impl TableMetadata {
    pub fn from_columns(table_name: &str, cols: Vec<IntrospectedColumn>) -> Self {
        let mut pk_col = None;
        let mut pk_is_int = false;
        let mut pk_is_auto_inc = false;

        for c in &cols {
            if c.is_pk {
                pk_col = Some(c.name.clone());
                let t = c.data_type.to_lowercase();
                let ct = c.column_type.to_lowercase();
                pk_is_int = t.contains("int") || ct.contains("int") || t.contains("serial");
                pk_is_auto_inc = c.is_auto_increment;
                break;
            }
        }

        // Fallback jika tidak ada deklarasi explicit PK
        if pk_col.is_none() {
            if let Some(c) = cols.iter().find(|c| c.name.eq_ignore_ascii_case("id")) {
                pk_col = Some(c.name.clone());
                let t = c.data_type.to_lowercase();
                let ct = c.column_type.to_lowercase();
                pk_is_int = t.contains("int") || ct.contains("int") || t.contains("serial");
                pk_is_auto_inc = c.is_auto_increment;
            } else if let Some(first) = cols.first() {
                pk_col = Some(first.name.clone());
                let t = first.data_type.to_lowercase();
                let ct = first.column_type.to_lowercase();
                pk_is_int = t.contains("int") || ct.contains("int") || t.contains("serial");
                pk_is_auto_inc = first.is_auto_increment;
            }
        }

        let pk_column = pk_col.unwrap_or_else(|| "id".to_string());
        let has_col = |name: &str| cols.iter().any(|c| c.name.eq_ignore_ascii_case(name));

        Self {
            table_name: table_name.to_string(),
            pk_column,
            pk_is_int,
            pk_is_auto_increment: pk_is_auto_inc,
            has_created: has_col("created"),
            has_updated: has_col("updated"),
            has_created_at: has_col("created_at"),
            has_updated_at: has_col("updated_at"),
            has_created_by: has_col("created_by") || has_col("createdBy"),
            has_updated_by: has_col("updated_by") || has_col("updatedBy"),
            has_collection_id: has_col("collectionId") || has_col("collection_id"),
            has_collection_name: has_col("collectionName") || has_col("collection_name"),
            columns: cols,
        }
    }

    pub fn has_column(&self, name: &str) -> bool {
        self.columns.iter().any(|c| c.name.eq_ignore_ascii_case(name))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectedRelation {
    pub column: String,          // e.g. "siswa_id"
    pub relation_name: String,   // e.g. "siswa"
    pub target_table: String,    // e.g. "siswa" or "siswas"
    pub target_pk: String,       // e.g. "id"
    pub display_col: String,     // e.g. "nama" or "name"
}

/// Pick appropriate display column for a table (e.g. nama, name, title, username) matching Padi Rust
pub fn pick_display_column(target_tbl: &str, target_cols: &[String]) -> String {
    let is_user_table = target_tbl == "users" || target_tbl.ends_with("_users");
    if is_user_table && target_cols.iter().any(|c| c.eq_ignore_ascii_case("username")) {
        return "username".to_string();
    }

    let candidates = [
        "name",
        "nama",
        "pen_name",
        "username",
        "title",
        "judul",
        "display_name",
        "full_name",
        "first_name",
        "nickname",
        "label",
        "caption",
        "content",
        "body",
        "message",
        "code",
        "slug",
        "email",
        "description",
    ];
    for cand in candidates {
        if let Some(col) = target_cols.iter().find(|c| c.eq_ignore_ascii_case(cand)) {
            return col.clone();
        }
    }

    if let Some(col) = target_cols.iter().find(|c| {
        let l = c.to_ascii_lowercase();
        (l.ends_with("_name") || l.ends_with("_title") || l.ends_with("_label"))
            && !l.contains("file") && !l.contains("image") && !l.contains("icon")
    }) {
        return col.clone();
    }

    if let Some(col) = target_cols.iter().find(|c| !c.eq_ignore_ascii_case("id") && !c.ends_with("_id")) {
        return col.clone();
    }

    "id".to_string()
}

/// Detect foreign key relations across SQLite, PostgreSQL, and MySQL/MariaDB with convention fallbacks.
pub async fn get_table_relations(pool: &DbPool, table_name: &str) -> Result<Vec<DetectedRelation>> {
    // Fast path: Check tenant in-memory relations cache first
    if let Some(rels) = pool.get_cached_relations(table_name) {
        return Ok(rels);
    }

    let mut relations = Vec::new();
    let mut handled_cols = std::collections::HashSet::new();

    // 1. Check physical database foreign keys
    let fk_rows = if pool.is_sqlite() {
        let sql = format!("PRAGMA foreign_key_list(`{}`);", table_name);
        pool.fetch_all(&sql, &[]).await.unwrap_or_default()
    } else if pool.is_postgres() {
        let sql = r#"
            SELECT kcu.column_name AS "from", ccu.table_name AS "table", ccu.column_name AS "to"
            FROM information_schema.table_constraints AS tc
            JOIN information_schema.key_column_usage AS kcu 
              ON tc.constraint_name = kcu.constraint_name AND tc.table_schema = kcu.table_schema
            JOIN information_schema.constraint_column_usage AS ccu 
              ON ccu.constraint_name = tc.constraint_name AND ccu.table_schema = tc.table_schema
            WHERE tc.constraint_type = 'FOREIGN KEY' AND tc.table_name = ?
        "#;
        pool.fetch_all(sql, &[Val::from(table_name)]).await.unwrap_or_default()
    } else {
        // MySQL / MariaDB
        let sql = r#"
            SELECT COLUMN_NAME AS `from`, REFERENCED_TABLE_NAME AS `table`, REFERENCED_COLUMN_NAME AS `to`
            FROM INFORMATION_SCHEMA.KEY_COLUMN_USAGE
            WHERE TABLE_SCHEMA = DATABASE() AND TABLE_NAME = ? AND REFERENCED_TABLE_NAME IS NOT NULL
        "#;
        pool.fetch_all(sql, &[Val::from(table_name)]).await.unwrap_or_default()
    };

    for r in fk_rows {
        let from_col = json_get_str(&r, "from");
        let from_col = if from_col.is_empty() { json_get_str(&r, "FROM") } else { from_col };
        let target_tbl = json_get_str(&r, "table");
        let target_tbl = if target_tbl.is_empty() { json_get_str(&r, "TABLE") } else { target_tbl };
        let to_col = json_get_str(&r, "to");
        let to_col = if to_col.is_empty() { json_get_str(&r, "TO") } else { to_col };
        let to_col = if to_col.is_empty() { "id".to_string() } else { to_col };

        if !from_col.is_empty() && !target_tbl.is_empty() {
            let rel_name = if let Some(stripped) = from_col.strip_suffix("_id") {
                stripped.to_string()
            } else if let Some(stripped) = from_col.strip_suffix("Id") {
                stripped.to_string()
            } else if from_col == "id" {
                target_tbl.clone()
            } else {
                from_col.clone()
            };

            let target_cols = get_existing_table_columns(pool, &target_tbl).await.unwrap_or_default();
            let display_col = pick_display_column(&target_tbl, &target_cols);

            relations.push(DetectedRelation {
                column: from_col.clone(),
                relation_name: rel_name,
                target_table: target_tbl,
                target_pk: to_col,
                display_col,
            });
            handled_cols.insert(from_col);
        }
    }

    // 2. Check explicit schema relation definitions from collection schema metadata
    if let Ok(Some(col_def)) = get_collection_by_name_or_id(pool, table_name).await {
        for field in &col_def.schema {
            if (field.field_type == FieldType::Relation || field.options.get("targetTable").is_some())
                && !handled_cols.contains(&field.name)
            {
                let opt_target = field
                    .options
                    .get("targetTable")
                    .or_else(|| field.options.get("collectionId"))
                    .and_then(|v| v.as_str());

                if let Some(target_tbl) = opt_target {
                    if !target_tbl.is_empty() {
                        let target_pk = field
                            .options
                            .get("targetField")
                            .and_then(|v| v.as_str())
                            .unwrap_or("id")
                            .to_string();

                        let rel_name = if let Some(stripped) = field.name.strip_suffix("_id") {
                            stripped.to_string()
                        } else if let Some(stripped) = field.name.strip_suffix("Id") {
                            stripped.to_string()
                        } else {
                            field.name.clone()
                        };

                        let target_cols = get_existing_table_columns(pool, target_tbl).await.unwrap_or_default();
                        let display_col = pick_display_column(target_tbl, &target_cols);

                        relations.push(DetectedRelation {
                            column: field.name.clone(),
                            relation_name: rel_name,
                            target_table: target_tbl.to_string(),
                            target_pk,
                            display_col,
                        });
                        handled_cols.insert(field.name.clone());
                    }
                }
            }
        }
    }

    // 3. Convention Fallback (e.g. siswa_id -> siswa / siswas)
    let all_tables = list_physical_tables(pool).await.unwrap_or_default();
    let current_cols = get_existing_table_columns(pool, table_name).await.unwrap_or_default();

    for col in current_cols {
        if handled_cols.contains(&col) {
            continue;
        }

        let cand_name = if let Some(stripped) = col.strip_suffix("_id") {
            Some(stripped)
        } else if let Some(stripped) = col.strip_suffix("Id") {
            Some(stripped)
        } else {
            None
        };

        if let Some(base_name) = cand_name {
            let matched_tbl = all_tables.iter().find(|t| {
                t.eq_ignore_ascii_case(base_name)
                    || t.eq_ignore_ascii_case(&format!("{}s", base_name))
                    || t.eq_ignore_ascii_case(&format!("{}es", base_name))
            });

            if let Some(target_tbl) = matched_tbl {
                let target_cols = get_existing_table_columns(pool, target_tbl).await.unwrap_or_default();
                let target_meta = get_table_metadata(pool, target_tbl).await.ok();
                let target_pk = target_meta.map(|m| m.pk_column).unwrap_or_else(|| "id".to_string());
                let display_col = pick_display_column(target_tbl, &target_cols);

                relations.push(DetectedRelation {
                    column: col.clone(),
                    relation_name: base_name.to_string(),
                    target_table: target_tbl.clone(),
                    target_pk,
                    display_col,
                });
                handled_cols.insert(col);
            }
        }
    }

    pool.set_cached_relations(table_name, relations.clone());
    Ok(relations)
}

/// Get complete table metadata (all fields + primary key information) for any supported database engine.
pub async fn get_table_metadata(pool: &DbPool, table_name: &str) -> Result<TableMetadata> {
    // Fast path: Check tenant in-memory metadata cache first
    if let Some(meta) = pool.get_cached_metadata(table_name) {
        return Ok(meta);
    }

    let cols = introspect_table_columns(pool, table_name).await?;
    let meta = TableMetadata::from_columns(table_name, cols);
    pool.set_cached_metadata(table_name, meta.clone());
    Ok(meta)
}

/// List all physical tables in the current database, excluding internal tables starting with `_`.
pub async fn list_physical_tables(pool: &DbPool) -> Result<Vec<String>> {
    let sql = if pool.is_sqlite() {
        "SELECT name FROM sqlite_master WHERE type IN ('table', 'view') AND name NOT LIKE 'sqlite_%' ORDER BY name ASC;"
    } else if pool.is_postgres() {
        "SELECT table_name FROM information_schema.tables WHERE table_schema = 'public' AND table_type IN ('BASE TABLE', 'VIEW') ORDER BY table_name ASC;"
    } else {
        // MySQL / MariaDB
        "SELECT table_name FROM information_schema.tables WHERE table_schema = DATABASE() AND table_type IN ('BASE TABLE', 'VIEW') ORDER BY table_name ASC;"
    };

    let rows = pool.fetch_all(sql, &[]).await?;
    let mut tables = Vec::new();
    for row in rows {
        let name = json_get_str(&row, "name");
        let name = if name.is_empty() {
            json_get_str(&row, "table_name")
        } else {
            name
        };
        let name = if name.is_empty() {
            json_get_str(&row, "TABLE_NAME")
        } else {
            name
        };
        if !name.is_empty() && !name.starts_with('_') {
            tables.push(name);
        }
    }
    Ok(tables)
}

/// Introspect detailed column info for a table across SQLite, PostgreSQL, and MySQL/MariaDB.
pub async fn introspect_table_columns(pool: &DbPool, table_name: &str) -> Result<Vec<IntrospectedColumn>> {
    let mut columns = Vec::new();

    if pool.is_sqlite() {
        let pragma_sql = format!("PRAGMA table_info(`{}`);", table_name);
        let rows = pool.fetch_all(&pragma_sql, &[]).await?;
        for r in rows {
            let name = json_get_str(&r, "name");
            if name.is_empty() {
                continue;
            }
            let data_type = json_get_str(&r, "type").to_lowercase();
            let notnull = json_get_i64(&r, "notnull") == 1;
            let pk = json_get_i64(&r, "pk") > 0;
            let is_auto_inc = pk && (data_type.contains("int"));
            let dflt = json_get_opt_str(&r, "dflt_value");
            columns.push(IntrospectedColumn {
                name,
                data_type: data_type.clone(),
                column_type: data_type,
                is_nullable: !notnull,
                is_unique: false,
                is_pk: pk,
                is_auto_increment: is_auto_inc,
                default_value: dflt,
            });
        }
    } else if pool.is_postgres() {
        let rows = pool
            .fetch_all(
                r#"
                SELECT 
                    c.column_name, 
                    c.data_type, 
                    c.udt_name, 
                    c.is_nullable, 
                    c.column_default,
                    CASE WHEN tc.constraint_type = 'PRIMARY KEY' THEN 1 ELSE 0 END AS is_pk,
                    CASE 
                        WHEN c.column_default LIKE 'nextval(%' 
                             OR c.data_type IN ('smallserial', 'serial', 'bigserial') 
                             OR c.is_identity = 'YES' THEN 1 
                        ELSE 0 
                    END AS is_auto_inc
                FROM information_schema.columns c
                LEFT JOIN information_schema.key_column_usage kcu
                    ON c.table_schema = kcu.table_schema
                    AND c.table_name = kcu.table_name
                    AND c.column_name = kcu.column_name
                LEFT JOIN information_schema.table_constraints tc
                    ON kcu.table_schema = tc.table_schema
                    AND kcu.table_name = tc.table_name
                    AND kcu.constraint_name = tc.constraint_name
                    AND tc.constraint_type = 'PRIMARY KEY'
                WHERE c.table_schema = 'public' AND c.table_name = ?
                ORDER BY c.ordinal_position ASC
                "#,
                &[Val::from(table_name)],
            )
            .await?;
        for r in rows {
            let name = json_get_str(&r, "column_name");
            if name.is_empty() {
                continue;
            }
            let data_type = json_get_str(&r, "data_type").to_lowercase();
            let col_type = json_get_str(&r, "udt_name").to_lowercase();
            let is_nullable = json_get_str(&r, "is_nullable").eq_ignore_ascii_case("YES");
            let dflt = json_get_opt_str(&r, "column_default");
            let is_pk = json_get_i64(&r, "is_pk") > 0;
            let is_auto_inc = json_get_i64(&r, "is_auto_inc") > 0;

            columns.push(IntrospectedColumn {
                name,
                data_type,
                column_type: col_type,
                is_nullable,
                is_unique: false,
                is_pk,
                is_auto_increment: is_auto_inc,
                default_value: dflt,
            });
        }
    } else {
        // MySQL / MariaDB
        let rows = pool
            .fetch_all(
                r#"
                SELECT COLUMN_NAME, DATA_TYPE, COLUMN_TYPE, IS_NULLABLE, COLUMN_KEY, EXTRA, COLUMN_DEFAULT
                FROM information_schema.columns
                WHERE TABLE_SCHEMA = DATABASE() AND TABLE_NAME = ?
                ORDER BY ORDINAL_POSITION ASC
                "#,
                &[Val::from(table_name)],
            )
            .await?;
        for r in rows {
            let name = json_get_str(&r, "COLUMN_NAME");
            let name = if name.is_empty() {
                json_get_str(&r, "column_name")
            } else {
                name
            };
            if name.is_empty() {
                continue;
            }
            let data_type = json_get_str(&r, "DATA_TYPE").to_lowercase();
            let data_type = if data_type.is_empty() {
                json_get_str(&r, "data_type").to_lowercase()
            } else {
                data_type
            };
            let col_type = json_get_str(&r, "COLUMN_TYPE").to_lowercase();
            let col_type = if col_type.is_empty() {
                json_get_str(&r, "column_type").to_lowercase()
            } else {
                col_type
            };
            let is_nullable_str = json_get_str(&r, "IS_NULLABLE");
            let is_nullable_str = if is_nullable_str.is_empty() {
                json_get_str(&r, "is_nullable")
            } else {
                is_nullable_str
            };
            let col_key = json_get_str(&r, "COLUMN_KEY");
            let col_key = if col_key.is_empty() {
                json_get_str(&r, "column_key")
            } else {
                col_key
            };
            let extra = json_get_str(&r, "EXTRA").to_lowercase();
            let extra = if extra.is_empty() {
                json_get_str(&r, "extra").to_lowercase()
            } else {
                extra
            };
            let dflt = json_get_opt_str(&r, "COLUMN_DEFAULT");
            let dflt = if dflt.is_none() {
                json_get_opt_str(&r, "column_default")
            } else {
                dflt
            };

            let is_nullable = is_nullable_str.eq_ignore_ascii_case("YES");
            let is_unique = col_key.eq_ignore_ascii_case("UNI");
            let is_pk = col_key.eq_ignore_ascii_case("PRI");
            let is_auto_inc = extra.contains("auto_increment");

            columns.push(IntrospectedColumn {
                name,
                data_type,
                column_type: col_type,
                is_nullable,
                is_unique,
                is_pk,
                is_auto_increment: is_auto_inc,
                default_value: dflt,
            });
        }
    }
    Ok(columns)
}

/// Introspect existing column names for a table across SQLite, PostgreSQL, and MySQL/MariaDB.
pub async fn get_existing_table_columns(pool: &DbPool, table_name: &str) -> Result<Vec<String>> {
    let cols = introspect_table_columns(pool, table_name).await?;
    Ok(cols.into_iter().map(|c| c.name).collect())
}

pub fn map_column_to_schema_field(col: &IntrospectedColumn) -> SchemaField {
    let type_lower = col.data_type.to_lowercase();
    let col_type_lower = col.column_type.to_lowercase();
    let name_lower = col.name.to_lowercase();

    let field_type = if col_type_lower.contains("tinyint(1)") || type_lower == "bool" || type_lower == "boolean" {
        FieldType::Bool
    } else if type_lower.contains("int")
        || type_lower.contains("float")
        || type_lower.contains("double")
        || type_lower.contains("decimal")
        || type_lower.contains("numeric")
        || type_lower.contains("real")
    {
        FieldType::Number
    } else if type_lower.contains("json") {
        FieldType::Json
    } else if type_lower.contains("date") || type_lower.contains("time") || type_lower.contains("year") {
        FieldType::Date
    } else if name_lower.contains("email") {
        FieldType::Email
    } else if name_lower.contains("url") || name_lower.contains("website") {
        FieldType::Url
    } else if name_lower.contains("avatar")
        || name_lower.contains("image")
        || name_lower.contains("photo")
        || name_lower.contains("file")
    {
        FieldType::File
    } else {
        FieldType::Text
    };

    let required = !col.is_nullable && col.default_value.is_none() && !col.is_pk;

    SchemaField {
        name: col.name.clone(),
        field_type,
        required,
        unique: col.is_unique,
        options: serde_json::json!({}),
    }
}

/// Automatically inspect physical tables in the database and synchronize them into `_collections`.
pub async fn sync_existing_tables(pool: &DbPool) -> Result<Vec<String>> {
    let tables = list_physical_tables(pool).await?;
    let mut synced_tables = Vec::new();
    let now = chrono::Utc::now().to_rfc3339();

    for table_name in &tables {
        let cols = introspect_table_columns(pool, table_name).await?;
        if cols.is_empty() {
            continue;
        }

        // Check if collection already exists in _collections
        if let Ok(Some(mut existing_col)) = get_collection_by_name_or_id(pool, table_name).await {
            let mut schema_updated = false;
            for col_info in &cols {
                if col_info.name.eq_ignore_ascii_case("id")
                    || col_info.name.eq_ignore_ascii_case("created")
                    || col_info.name.eq_ignore_ascii_case("updated")
                {
                    continue;
                }
                let already_has = existing_col
                    .schema
                    .iter()
                    .any(|f| f.name.eq_ignore_ascii_case(&col_info.name));
                if !already_has {
                    existing_col.schema.push(map_column_to_schema_field(col_info));
                    schema_updated = true;
                }
            }
            if schema_updated {
                existing_col.updated = now.clone();
                let _ = save_collection_metadata(pool, &existing_col).await;
                synced_tables.push(table_name.clone());
            }
            continue;
        }

        // Register new collection from physical table
        let mut schema_fields = Vec::new();
        let mut has_email = false;
        let mut has_password = false;

        for col_info in &cols {
            let col_name_lower = col_info.name.to_lowercase();
            if col_name_lower == "email" {
                has_email = true;
            }
            if col_name_lower.contains("password") {
                has_password = true;
            }

            // Exclude id, created, updated as base record attributes in PadiBase
            if col_info.name.eq_ignore_ascii_case("id")
                || col_info.name.eq_ignore_ascii_case("created")
                || col_info.name.eq_ignore_ascii_case("updated")
            {
                continue;
            }

            schema_fields.push(map_column_to_schema_field(col_info));
        }

        let is_auth = (has_email && has_password) || table_name.eq_ignore_ascii_case("users");

        let new_col = Collection {
            id: generate_id(),
            name: table_name.clone(),
            collection_type: if is_auth {
                CollectionType::Auth
            } else {
                CollectionType::Base
            },
            schema: schema_fields,
            realtime: true,
            auto_load_relations: false,
            blameable: true,
            list_rule: Some("".to_string()),
            all_rule: Some("".to_string()),
            view_rule: Some("".to_string()),
            create_rule: Some("@request.auth.id != \"\"".to_string()),
            update_rule: Some("@request.auth.id != \"\"".to_string()),
            delete_rule: Some("@request.auth.id != \"\"".to_string()),
            resource_fields: None,
            resource_relations: None,
            resource_relation_fields: None,
            list_fields: None,
            all_fields: None,
            view_fields: None,
            list_relations: None,
            all_relations: None,
            view_relations: None,
            created: now.clone(),
            updated: now.clone(),
        };

        save_collection_metadata(pool, &new_col).await?;
        synced_tables.push(table_name.clone());
    }

    // Pre-warm in-memory cache for all physical tables (metadata, relations, collections)
    for table_name in &tables {
        let _ = get_table_metadata(pool, table_name).await;
        let _ = get_table_relations(pool, table_name).await;
        let _ = get_collection_by_name_or_id(pool, table_name).await;
    }

    Ok(synced_tables)
}

fn sql_type_for_field(field_type: &FieldType, unique: bool) -> &'static str {
    match field_type {
        FieldType::Number => "DOUBLE PRECISION",
        FieldType::Bool => "INTEGER",
        _ => {
            if unique {
                "VARCHAR(255)"
            } else {
                "TEXT"
            }
        }
    }
}

/// Dynamically determine the matching physical SQL data type for a field.
/// If field is `Relation`, introspects the target referenced table to match its exact type (e.g. INT vs BIGINT vs VARCHAR).
pub async fn resolve_field_sql_type(pool: &DbPool, field: &SchemaField) -> String {
    if field.field_type == FieldType::Relation {
        let opt_target = field
            .options
            .get("targetTable")
            .or_else(|| field.options.get("collectionId"))
            .and_then(|v| v.as_str());

        let target_tbl = match opt_target {
            Some(t) if !t.is_empty() => Some(t.to_string()),
            _ => {
                if let Some(stripped) = field
                    .name
                    .strip_suffix("_id")
                    .or_else(|| field.name.strip_suffix("Id"))
                {
                    let all_tables = list_physical_tables(pool).await.unwrap_or_default();
                    all_tables.into_iter().find(|t| {
                        t.eq_ignore_ascii_case(stripped)
                            || t.eq_ignore_ascii_case(&format!("{}s", stripped))
                            || t.eq_ignore_ascii_case(&format!("{}es", stripped))
                    })
                } else {
                    None
                }
            }
        };

        if let Some(tbl) = target_tbl {
            let target_pk = field
                .options
                .get("targetField")
                .and_then(|v| v.as_str())
                .unwrap_or("id");

            if let Ok(cols) = introspect_table_columns(pool, &tbl).await {
                if let Some(col_info) = cols.iter().find(|c| c.name.eq_ignore_ascii_case(target_pk)) {
                    let t_lower = col_info.data_type.to_lowercase();
                    let ct_lower = col_info.column_type.to_lowercase();

                    if ct_lower.contains("bigint") || t_lower == "int8" || t_lower == "bigint" {
                        return if pool.is_sqlite() { "INTEGER".to_string() } else { "BIGINT".to_string() };
                    } else if ct_lower.contains("smallint") || t_lower == "int2" || t_lower == "smallint" {
                        return if pool.is_sqlite() { "INTEGER".to_string() } else { "SMALLINT".to_string() };
                    } else if ct_lower.contains("int") || t_lower == "int4" || t_lower == "integer" {
                        return if pool.is_sqlite() { "INTEGER".to_string() } else { "INT".to_string() };
                    } else if t_lower.contains("uuid") {
                        return "UUID".to_string();
                    } else if !col_info.column_type.is_empty() {
                        return col_info.column_type.clone();
                    }
                }
            }
        }

        if field.unique {
            "VARCHAR(255)".to_string()
        } else {
            "TEXT".to_string()
        }
    } else {
        if let Some(explicit_type) = field
            .options
            .get("sqlType")
            .or_else(|| field.options.get("nativeType"))
            .and_then(|v| v.as_str())
        {
            if !explicit_type.is_empty() {
                let mut resolved = explicit_type.to_string();
                if resolved.eq_ignore_ascii_case("varchar") {
                    let len = field
                        .options
                        .get("length")
                        .or_else(|| field.options.get("maxLength"))
                        .or_else(|| field.options.get("max"))
                        .and_then(|v| v.as_i64())
                        .unwrap_or(255);
                    resolved = format!("VARCHAR({})", len);
                }
                return resolved;
            }
        }

        let custom_varchar_len = field
            .options
            .get("length")
            .or_else(|| field.options.get("maxLength"))
            .or_else(|| field.options.get("max"))
            .and_then(|v| v.as_i64())
            .unwrap_or(255);

        if pool.is_sqlite() {
            match field.field_type {
                FieldType::Number => "REAL".to_string(),
                FieldType::Bool => "INTEGER".to_string(),
                FieldType::Json => "TEXT".to_string(),
                FieldType::Date => "TEXT".to_string(),
                _ => "TEXT".to_string(),
            }
        } else if pool.is_postgres() {
            match field.field_type {
                FieldType::Number => "DOUBLE PRECISION".to_string(),
                FieldType::Bool => "BOOLEAN".to_string(),
                FieldType::Json => "JSONB".to_string(),
                FieldType::Date => "TIMESTAMPTZ".to_string(),
                _ => {
                    if field.unique {
                        format!("VARCHAR({})", custom_varchar_len)
                    } else {
                        "TEXT".to_string()
                    }
                }
            }
        } else if pool.is_mysql() {
            match field.field_type {
                FieldType::Number => "DOUBLE".to_string(),
                FieldType::Bool => "TINYINT(1)".to_string(),
                FieldType::Json => "JSON".to_string(),
                FieldType::Date => "DATETIME".to_string(),
                _ => {
                    if field.unique {
                        format!("VARCHAR({})", custom_varchar_len)
                    } else {
                        "TEXT".to_string()
                    }
                }
            }
        } else {
            sql_type_for_field(&field.field_type, field.unique).to_string()
        }
    }
}

pub async fn create_or_update_collection_table(pool: &DbPool, col: &Collection) -> Result<()> {
    let physical_tables = list_physical_tables(pool).await.unwrap_or_default();

    // 1. Create table if not exists with base columns
    let mut columns_sql = vec![
        "`id` VARCHAR(255) PRIMARY KEY NOT NULL".to_string(),
    ];

    if col.collection_type != CollectionType::Auth {
        columns_sql.push("`created_at` BIGINT".to_string());
        columns_sql.push("`updated_at` BIGINT".to_string());
        if col.blameable {
            columns_sql.push("`created_by` VARCHAR(255)".to_string());
            columns_sql.push("`updated_by` VARCHAR(255)".to_string());
        }
    }

    if col.collection_type == CollectionType::Auth {
        // Standard Padi User fields (compatible with padi_rust, padi_go, padi_php)
        columns_sql.push("`name` VARCHAR(255)".to_string());
        columns_sql.push("`username` VARCHAR(50)".to_string());
        columns_sql.push("`email` VARCHAR(255) UNIQUE NOT NULL".to_string());
        columns_sql.push("`password` VARCHAR(255)".to_string());
        columns_sql.push("`passwordHash` TEXT".to_string());
        columns_sql.push("`role` VARCHAR(50) DEFAULT 'user'".to_string());
        columns_sql.push("`status` VARCHAR(20) DEFAULT 'active'".to_string());
        columns_sql.push("`emailVisibility` INTEGER DEFAULT 0".to_string());
        columns_sql.push("`verified` INTEGER DEFAULT 0".to_string());
        columns_sql.push("`email_verified_at` BIGINT".to_string());
        columns_sql.push("`remember_token` VARCHAR(100)".to_string());
        columns_sql.push("`last_login_at` VARCHAR(64)".to_string());
        columns_sql.push("`created_by` VARCHAR(255)".to_string());
        columns_sql.push("`updated_by` VARCHAR(255)".to_string());
        columns_sql.push("`created_at` VARCHAR(64)".to_string());
        columns_sql.push("`updated_at` VARCHAR(64)".to_string());
        columns_sql.push("`tokenKey` VARCHAR(255) NOT NULL".to_string());
    }

    let mut fk_constraints = Vec::new();

    for field in &col.schema {
        if columns_sql.iter().any(|c| c.starts_with(&format!("`{}` ", field.name))) {
            continue;
        }
        let col_type = resolve_field_sql_type(pool, field).await;
        let nullability = if field.required { "NOT NULL" } else { "" };
        let unique = if field.unique { "UNIQUE" } else { "" };
        columns_sql.push(
            format!("`{}` {} {} {}", field.name, col_type, nullability, unique)
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" "),
        );

        if field.field_type == FieldType::Relation {
            let opt_target = field
                .options
                .get("targetTable")
                .or_else(|| field.options.get("collectionId"))
                .and_then(|v| v.as_str());

            let target_tbl = match opt_target {
                Some(t) if !t.is_empty() => Some(t.to_string()),
                _ => {
                    if let Some(stripped) = field
                        .name
                        .strip_suffix("_id")
                        .or_else(|| field.name.strip_suffix("Id"))
                    {
                        physical_tables.iter().find(|t| {
                            t.eq_ignore_ascii_case(stripped)
                                || t.eq_ignore_ascii_case(&format!("{}s", stripped))
                                || t.eq_ignore_ascii_case(&format!("{}es", stripped))
                        }).cloned()
                    } else {
                        None
                    }
                }
            };

            if let Some(tbl) = target_tbl {
                if physical_tables.iter().any(|t| t.eq_ignore_ascii_case(&tbl)) && !tbl.eq_ignore_ascii_case(&col.name) {
                    let target_pk = field
                        .options
                        .get("targetField")
                        .and_then(|v| v.as_str())
                        .unwrap_or("id");
                    let on_delete = if field.required { "CASCADE" } else { "SET NULL" };
                    fk_constraints.push(format!(
                        "FOREIGN KEY (`{}`) REFERENCES `{}`(`{}`) ON DELETE {}",
                        field.name, tbl, target_pk, on_delete
                    ));
                }
            }
        }
    }

    let mut full_create_parts = columns_sql.clone();
    full_create_parts.extend(fk_constraints);

    let create_sql = format!(
        "CREATE TABLE IF NOT EXISTS `{}` ({});",
        col.name,
        full_create_parts.join(", ")
    );

    pool.execute(&create_sql, &[]).await?;

    // 2. Add missing columns if table already existed (ALTER TABLE across SQLite / MySQL / MariaDB / Postgres)
    let existing_cols = get_existing_table_columns(pool, &col.name).await?;
    let has_col = |target: &str| existing_cols.iter().any(|c| c.eq_ignore_ascii_case(target));

    if col.collection_type == CollectionType::Auth {
        let auth_extra_cols = [
            ("name", "VARCHAR(255)"),
            ("username", "VARCHAR(50)"),
            ("password", "VARCHAR(255)"),
            ("passwordHash", "TEXT"),
            ("role", "VARCHAR(50) DEFAULT 'user'"),
            ("status", "VARCHAR(20) DEFAULT 'active'"),
            ("emailVisibility", "INTEGER DEFAULT 0"),
            ("verified", "INTEGER DEFAULT 0"),
            ("email_verified_at", "BIGINT"),
            ("remember_token", "VARCHAR(100)"),
            ("last_login_at", "VARCHAR(64)"),
            ("created_by", "VARCHAR(255)"),
            ("updated_by", "VARCHAR(255)"),
            ("created_at", "VARCHAR(64)"),
            ("updated_at", "VARCHAR(64)"),
            ("tokenKey", "VARCHAR(255)"),
        ];

        for (col_name, col_def) in auth_extra_cols {
            if !has_col(col_name) {
                let _ = pool
                    .execute(
                        &format!("ALTER TABLE `{}` ADD COLUMN `{}` {};", col.name, col_name, col_def),
                        &[],
                    )
                    .await;
            }
        }
    } else {
        if !has_col("created_at") {
            let _ = pool
                .execute(
                    &format!("ALTER TABLE `{}` ADD COLUMN `created_at` BIGINT;", col.name),
                    &[],
                )
                .await;
        }
        if !has_col("updated_at") {
            let _ = pool
                .execute(
                    &format!("ALTER TABLE `{}` ADD COLUMN `updated_at` BIGINT;", col.name),
                    &[],
                )
                .await;
        }
        if col.blameable {
            if !has_col("created_by") {
                let _ = pool
                    .execute(
                        &format!("ALTER TABLE `{}` ADD COLUMN `created_by` VARCHAR(255);", col.name),
                        &[],
                    )
                    .await;
            }
            if !has_col("updated_by") {
                let _ = pool
                    .execute(
                        &format!("ALTER TABLE `{}` ADD COLUMN `updated_by` VARCHAR(255);", col.name),
                        &[],
                    )
                    .await;
            }
        }
    }

    for field in &col.schema {
        if !has_col(&field.name) {
            let col_type = resolve_field_sql_type(pool, field).await;
            let alter_sql = format!(
                "ALTER TABLE `{}` ADD COLUMN `{}` {};",
                col.name, field.name, col_type
            );
            let _ = pool.execute(&alter_sql, &[]).await;

            // Attempt to add physical foreign key for MySQL / PostgreSQL if target table exists
            if field.field_type == FieldType::Relation && (pool.is_mysql() || pool.is_postgres()) {
                let opt_target = field
                    .options
                    .get("targetTable")
                    .or_else(|| field.options.get("collectionId"))
                    .and_then(|v| v.as_str());

                let target_tbl = match opt_target {
                    Some(t) if !t.is_empty() => Some(t.to_string()),
                    _ => {
                        if let Some(stripped) = field
                            .name
                            .strip_suffix("_id")
                            .or_else(|| field.name.strip_suffix("Id"))
                        {
                            physical_tables.iter().find(|t| {
                                t.eq_ignore_ascii_case(stripped)
                                    || t.eq_ignore_ascii_case(&format!("{}s", stripped))
                                    || t.eq_ignore_ascii_case(&format!("{}es", stripped))
                            }).cloned()
                        } else {
                            None
                        }
                    }
                };

                if let Some(tbl) = target_tbl {
                    if physical_tables.iter().any(|t| t.eq_ignore_ascii_case(&tbl)) && !tbl.eq_ignore_ascii_case(&col.name) {
                        let target_pk = field
                            .options
                            .get("targetField")
                            .and_then(|v| v.as_str())
                            .unwrap_or("id");
                        let on_delete = if field.required { "CASCADE" } else { "SET NULL" };
                        let fk_sql = format!(
                            "ALTER TABLE `{}` ADD CONSTRAINT `fk_{}_{}` FOREIGN KEY (`{}`) REFERENCES `{}`(`{}`) ON DELETE {};",
                            col.name, col.name, field.name, field.name, tbl, target_pk, on_delete
                        );
                        let _ = pool.execute(&fk_sql, &[]).await;
                    }
                }
            }
        }
    }

    pool.invalidate_schema_cache(&col.name);
    pool.invalidate_schema_cache(&col.id);

    Ok(())
}

// ============================================================================
// DYNAMIC CUSTOM SQL ENDPOINTS (`_endpoints`)
// ============================================================================

fn row_to_custom_endpoint(row: &Value) -> CustomEndpoint {
    CustomEndpoint {
        id: json_get_str(row, "id"),
        name: json_get_str(row, "name"),
        path: json_get_str(row, "path")
            .trim_matches('/')
            .to_string(),
        method: {
            let m = json_get_str(row, "method").to_uppercase();
            if m.is_empty() {
                "GET".to_string()
            } else {
                m
            }
        },
        sql: json_get_str(row, "sql"),
        description: json_get_str(row, "description"),
        auth_rule: {
            let r = json_get_str(row, "authRule");
            if r.is_empty() {
                "public".to_string()
            } else {
                r
            }
        },
        transaction: json_get_i64(row, "transaction") != 0,
        single_row: json_get_i64(row, "singleRow") != 0,
        created: json_get_str(row, "created"),
        updated: json_get_str(row, "updated"),
    }
}

pub async fn get_all_custom_endpoints(pool: &DbPool) -> Result<Vec<CustomEndpoint>> {
    let rows = pool
        .fetch_all(
            "SELECT * FROM `_endpoints` ORDER BY `created` ASC;",
            &[],
        )
        .await?;

    let mut list = Vec::with_capacity(rows.len());
    if let Ok(mut cache) = pool.cache.endpoints.write() {
        cache.clear();
        for r in &rows {
            let ep = row_to_custom_endpoint(r);
            let clean = ep.path.trim().trim_matches('/').to_lowercase();
            cache.insert(clean.clone(), ep.clone());
            if let Some(without_api) = clean.strip_prefix("api/") {
                cache.insert(without_api.to_string(), ep.clone());
            } else {
                cache.insert(format!("api/{}", clean), ep.clone());
            }
            if let Some(without_custom) = clean.strip_prefix("api/custom/") {
                cache.insert(without_custom.to_string(), ep.clone());
                cache.insert(format!("custom/{}", without_custom), ep.clone());
            } else if let Some(without_custom) = clean.strip_prefix("custom/") {
                cache.insert(without_custom.to_string(), ep.clone());
                cache.insert(format!("api/custom/{}", without_custom), ep.clone());
            }
            cache.insert(ep.id.clone(), ep.clone());
            list.push(ep);
        }
    } else {
        for r in &rows {
            list.push(row_to_custom_endpoint(r));
        }
    }
    Ok(list)
}

pub async fn get_custom_endpoint_by_path_or_id(
    pool: &DbPool,
    path_or_id: &str,
) -> Result<Option<CustomEndpoint>> {
    let clean = path_or_id.trim().trim_matches('/');
    if clean.is_empty() {
        return Ok(None);
    }

    let clean_lower = clean.to_lowercase();
    let without_api = clean_lower.strip_prefix("api/").unwrap_or(&clean_lower).to_string();
    let with_api = format!("api/{}", without_api);

    if let Ok(cache) = pool.cache.endpoints.read() {
        for candidate in [&clean_lower, &without_api, &with_api, &clean.to_string()] {
            if let Some(ep) = cache.get(candidate) {
                return Ok(Some(ep.clone()));
            }
        }
    }

    let row = pool
        .fetch_optional(
            "SELECT * FROM `_endpoints` WHERE LOWER(`path`) = LOWER(?) OR LOWER(`path`) = LOWER(?) OR `id` = ? LIMIT 1;",
            &[Val::from(clean), Val::from(without_api.as_str()), Val::from(clean)],
        )
        .await?;

    if let Some(r) = row {
        let ep = row_to_custom_endpoint(&r);
        if let Ok(mut cache) = pool.cache.endpoints.write() {
            cache.insert(ep.path.to_lowercase(), ep.clone());
            cache.insert(ep.id.clone(), ep.clone());
        }
        Ok(Some(ep))
    } else {
        Ok(None)
    }
}

pub async fn save_custom_endpoint(
    pool: &DbPool,
    ep: &CustomEndpoint,
) -> Result<CustomEndpoint> {
    let clean_path = ep
        .path
        .trim()
        .trim_matches('/')
        .to_string();

    if clean_path.is_empty() {
        return Err(anyhow!("Endpoint path cannot be empty."));
    }
    if ep.sql.trim().is_empty() {
        return Err(anyhow!("Endpoint SQL query cannot be empty."));
    }

    let now = chrono::Utc::now().to_rfc3339();
    let method = ep.method.trim().to_uppercase();

    // If `ep.id` is empty, check if an endpoint with the same `path` already exists so it overrides cleanly
    let existing_by_path = if ep.id.trim().is_empty() {
        get_custom_endpoint_by_path_or_id(pool, &clean_path).await.ok().flatten()
    } else {
        None
    };

    let id = if !ep.id.trim().is_empty() {
        ep.id.trim().to_string()
    } else if let Some(ref ex) = existing_by_path {
        ex.id.clone()
    } else {
        generate_id()
    };

    let created = if !ep.created.trim().is_empty() {
        ep.created.clone()
    } else if let Some(ref ex) = existing_by_path {
        ex.created.clone()
    } else {
        now.clone()
    };
    let tx_int = if ep.transaction { 1i64 } else { 0i64 };
    let single_int = if ep.single_row { 1i64 } else { 0i64 };

    let sql = if pool.is_mysql() {
        r#"
        INSERT INTO `_endpoints`
            (`id`, `name`, `path`, `method`, `sql`, `description`, `authRule`, `transaction`, `singleRow`, `created`, `updated`)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        ON DUPLICATE KEY UPDATE
            `name` = VALUES(`name`),
            `path` = VALUES(`path`),
            `method` = VALUES(`method`),
            `sql` = VALUES(`sql`),
            `description` = VALUES(`description`),
            `authRule` = VALUES(`authRule`),
            `transaction` = VALUES(`transaction`),
            `singleRow` = VALUES(`singleRow`),
            `updated` = VALUES(`updated`);
        "#
    } else {
        r#"
        INSERT INTO `_endpoints`
            (`id`, `name`, `path`, `method`, `sql`, `description`, `authRule`, `transaction`, `singleRow`, `created`, `updated`)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        ON CONFLICT(`id`) DO UPDATE SET
            `name` = excluded.`name`,
            `path` = excluded.`path`,
            `method` = excluded.`method`,
            `sql` = excluded.`sql`,
            `description` = excluded.`description`,
            `authRule` = excluded.`authRule`,
            `transaction` = excluded.`transaction`,
            `singleRow` = excluded.`singleRow`,
            `updated` = excluded.`updated`;
        "#
    };

    pool.execute(
        sql,
        &[
            Val::from(&id),
            Val::from(&ep.name),
            Val::from(&clean_path),
            Val::from(&method),
            Val::from(&ep.sql),
            Val::from(&ep.description),
            Val::from(&ep.auth_rule),
            Val::from(tx_int),
            Val::from(single_int),
            Val::from(&created),
            Val::from(&now),
        ],
    )
    .await?;

    let _ = get_all_custom_endpoints(pool).await;

    get_custom_endpoint_by_path_or_id(pool, &id)
        .await?
        .ok_or_else(|| anyhow!("Failed to load saved custom endpoint"))
}

pub async fn delete_custom_endpoint(pool: &DbPool, id_or_path: &str) -> Result<bool> {
    let clean = id_or_path.trim().trim_matches('/');
    let (affected, _) = pool
        .execute(
            "DELETE FROM `_endpoints` WHERE `id` = ? OR LOWER(`path`) = LOWER(?);",
            &[Val::from(clean), Val::from(clean)],
        )
        .await?;
    let _ = get_all_custom_endpoints(pool).await;
    Ok(affected > 0)
}

/// Match an incoming HTTP request against any user-defined Override Endpoint in RAM cache.
/// Supports exact path matches (e.g. `api/collections/siswa/records` or `collections/siswa/records`)
/// as well as path parameter templates (e.g. `api/collections/siswa/records/{id}` or `:id`).
pub fn match_override_endpoint(
    pool: &DbPool,
    http_method: &str,
    request_path: &str,
) -> Option<(CustomEndpoint, HashMap<String, String>)> {
    let norm = request_path.trim().trim_matches('/');
    let lower = norm.to_lowercase();

    // Never override core admin/endpoint management routes so the Dashboard cannot be locked out
    if lower.is_empty()
        || lower.starts_with("_padibase")
        || lower.starts_with("dashboard")
        || lower.starts_with("api/endpoints")
        || lower.starts_with("api/admins")
        || lower.starts_with("api/settings")
        || lower.starts_with("api/backups")
    {
        return None;
    }

    let cache = pool.cache.endpoints.read().ok()?;
    if cache.is_empty() {
        return None;
    }

    let method_upper = http_method.trim().to_uppercase();
    let without_api = lower.strip_prefix("api/").unwrap_or(&lower);

    // 1. Fast O(1) exact path lookup
    for candidate in [lower.as_str(), without_api] {
        if let Some(ep) = cache.get(candidate) {
            let ep_m = ep.method.trim().to_uppercase();
            if ep_m == method_upper || ep_m == "ANY" || ep_m == "*" {
                return Some((ep.clone(), HashMap::new()));
            }
        }
    }

    // 2. Pattern matching for routes with `{param}` or `:param` (e.g. `api/collections/siswa/records/{id}`)
    let req_segments: Vec<&str> = norm.split('/').collect();
    let req_no_api_segments: Vec<&str> = norm
        .strip_prefix("api/")
        .unwrap_or(norm)
        .split('/')
        .collect();

    for (k, ep) in cache.iter() {
        // Skip duplicate ID keys in cache (only check path keys that match `ep.path.to_lowercase()`)
        if k != &ep.path.to_lowercase() {
            continue;
        }
        if !ep.path.contains('{') && !ep.path.contains(':') {
            continue;
        }
        let ep_m = ep.method.trim().to_uppercase();
        if ep_m != method_upper && ep_m != "ANY" && ep_m != "*" {
            continue;
        }

        let pat_segments: Vec<&str> = ep.path.trim_matches('/').split('/').collect();
        for target_segs in [&req_segments, &req_no_api_segments] {
            if pat_segments.len() != target_segs.len() {
                continue;
            }
            let mut matched = true;
            let mut extracted = HashMap::new();
            for (p_seg, r_seg) in pat_segments.iter().zip(target_segs.iter()) {
                if let Some(name) = p_seg.strip_prefix('{').and_then(|s| s.strip_suffix('}')) {
                    extracted.insert(name.to_string(), (*r_seg).to_string());
                } else if let Some(name) = p_seg.strip_prefix(':') {
                    extracted.insert(name.to_string(), (*r_seg).to_string());
                } else if !p_seg.eq_ignore_ascii_case(r_seg) {
                    matched = false;
                    break;
                }
            }
            if matched {
                return Some((ep.clone(), extracted));
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_db_config_drivers() {
        let tmp = Path::new("./padi_data");

        let (url, drv) = resolve_db_config(tmp, Some("mysql://root:pass@localhost:3306/mydb"));
        assert_eq!(drv, DbDriver::MySql);
        assert_eq!(url, "mysql://root:pass@localhost:3306/mydb");

        let (url, drv) = resolve_db_config(tmp, Some("mariadb://root:pass@localhost:3306/mydb"));
        assert_eq!(drv, DbDriver::MariaDb);
        assert_eq!(url, "mysql://root:pass@localhost:3306/mydb");

        let (url, drv) = resolve_db_config(tmp, Some("postgres://postgres:pass@localhost:5432/mydb"));
        assert_eq!(drv, DbDriver::Postgres);
        assert_eq!(url, "postgres://postgres:pass@localhost:5432/mydb");

        let (url, drv) = resolve_db_config(tmp, Some("postgresql://postgres:pass@localhost:5432/mydb"));
        assert_eq!(drv, DbDriver::Postgres);
        assert_eq!(url, "postgresql://postgres:pass@localhost:5432/mydb");

        let (url, drv) = resolve_db_config(tmp, Some("sqlite://./test.db"));
        assert_eq!(drv, DbDriver::Sqlite);
        assert_eq!(url, "sqlite://./test.db");
    }

    #[test]
    fn test_parse_json_str_or_val() {
        let raw_obj = r#"{"id": 1, "username": "admin"}"#.to_string();
        let val = parse_json_str_or_val(raw_obj);
        assert!(val.is_object());
        assert_eq!(val["id"], 1);
        assert_eq!(val["username"], "admin");

        let raw_arr = r#"[1, 2, "three"]"#.to_string();
        let val2 = parse_json_str_or_val(raw_arr);
        assert!(val2.is_array());
        assert_eq!(val2[0], 1);

        let normal_str = "hello world".to_string();
        let val3 = parse_json_str_or_val(normal_str);
        assert_eq!(val3, Value::String("hello world".to_string()));
    }

    #[tokio::test]
    async fn test_sqlite_json_object_parse() {
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();

        let db_pool = DbPool {
            driver_pool: DbDriverPool::Sqlite(pool),
            cache: Arc::new(SchemaCache::default()),
        };

        let rows = db_pool
            .fetch_all("SELECT 1 AS id, JSON_OBJECT('id', 4, 'username', 'alina') AS user;", &[])
            .await
            .unwrap();

        assert_eq!(rows.len(), 1);
        let row = &rows[0];
        assert_eq!(row["id"], 1);
        // Ensure user is parsed as an Object, NOT an escaped string!
        assert!(row["user"].is_object(), "Expected user to be an Object, got: {:?}", row["user"]);
        assert_eq!(row["user"]["id"], 4);
        assert_eq!(row["user"]["username"], "alina");
    }
}
