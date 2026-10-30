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
use super::schema::{generate_id, Collection, CollectionType, FieldType, SchemaField};

/// Tenant/Database Schema Cache - populated once at startup, served entirely from RAM.
#[derive(Default, Debug)]
pub struct SchemaCache {
    pub table_metadata: RwLock<HashMap<String, TableMetadata>>,
    pub relations: RwLock<HashMap<String, Vec<DetectedRelation>>>,
    pub collections: RwLock<HashMap<String, Collection>>,
    pub settings: RwLock<Option<serde_json::Value>>,
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

pub fn record_query(sql: &str, params: &[Val], start: std::time::Instant) {
    let duration = start.elapsed();
    let ms = (duration.as_secs_f64() * 1000.0 * 100.0).round() / 100.0;
    let clean_sql = sql.split_whitespace().collect::<Vec<_>>().join(" ");
    let _ = REQUEST_QUERIES.try_with(|cell| {
        if let Ok(mut logs) = cell.lock() {
            logs.push(QueryLog {
                sql: clean_sql,
                bindings: params.iter().map(Value::from).collect(),
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
            v.map(|x| json!(x))
        } else if let Ok(v) = row.try_get::<Option<Vec<u8>>, _>(i) {
            v.map(|x| String::from_utf8(x).map(|s| json!(s)).unwrap_or(Value::Null))
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
            v.map(|x| json!(x))
        } else if let Ok(v) = row.try_get::<Option<Vec<u8>>, _>(i) {
            v.map(|x| String::from_utf8(x).map(|s| json!(s)).unwrap_or(Value::Null))
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
            v.map(|x| json!(x))
        } else if let Ok(v) = row.try_get::<Option<Vec<u8>>, _>(i) {
            v.map(|x| String::from_utf8(x).map(|s| json!(s)).unwrap_or(Value::Null))
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
                .busy_timeout(std::time::Duration::from_secs(5));

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
            `listRule` TEXT,
            `viewRule` TEXT,
            `createRule` TEXT,
            `updateRule` TEXT,
            `deleteRule` TEXT,
            `created` VARCHAR(64) NOT NULL,
            `updated` VARCHAR(64) NOT NULL
        );
        "#,
        &[],
    )
    .await?;

    // Ensure `realtime` and `autoLoadRelations` columns exist on existing `_collections` tables
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

    // Preload system settings into RAM cache once at startup
    let _ = get_system_settings(pool).await;

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
            list_rule: Some("".to_string()),
            view_rule: Some("".to_string()),
            create_rule: Some("".to_string()),
            update_rule: Some("id = @request.auth.id".to_string()),
            delete_rule: Some("id = @request.auth.id".to_string()),
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
            "appUrl": format!("http://{}:{}", Env::get_or("APP_HOST", "127.0.0.1"), Env::get_or("APP_PORT", "8090")),
            "hideControls": false
        }
    })
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
pub async fn save_system_settings(pool: &DbPool, settings: Value) -> Result<()> {
    let val_str = serde_json::to_string(&settings).unwrap_or_else(|_| "{}".to_string());
    let sql = if pool.is_mysql() {
        "INSERT INTO `_params` (`key`, `value`) VALUES ('settings', ?) ON DUPLICATE KEY UPDATE `value` = VALUES(`value`)"
    } else {
        "INSERT INTO `_params` (`key`, `value`) VALUES ('settings', ?) ON CONFLICT(`key`) DO UPDATE SET `value` = excluded.`value`"
    };

    pool.execute(sql, &[Val::from(&val_str)]).await?;

    // Reload into memory cache immediately
    pool.set_cached_settings(settings.clone());

    // Sync runtime APP_NAME if provided in settings
    if let Some(app_name) = settings.get("meta").and_then(|m| m.get("appName")).and_then(|n| n.as_str()) {
        if !app_name.trim().is_empty() {
            Env::set("APP_NAME", app_name);
        }
    }

    Ok(())
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

    let sql = if pool.is_mysql() {
        r#"
        INSERT INTO `_collections` (`id`, `name`, `type`, `schema`, `realtime`, `autoLoadRelations`, `listRule`, `viewRule`, `createRule`, `updateRule`, `deleteRule`, `created`, `updated`)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        ON DUPLICATE KEY UPDATE
            `name` = VALUES(`name`),
            `type` = VALUES(`type`),
            `schema` = VALUES(`schema`),
            `realtime` = VALUES(`realtime`),
            `autoLoadRelations` = VALUES(`autoLoadRelations`),
            `listRule` = VALUES(`listRule`),
            `viewRule` = VALUES(`viewRule`),
            `createRule` = VALUES(`createRule`),
            `updateRule` = VALUES(`updateRule`),
            `deleteRule` = VALUES(`deleteRule`),
            `updated` = VALUES(`updated`)
        "#
    } else {
        r#"
        INSERT INTO `_collections` (`id`, `name`, `type`, `schema`, `realtime`, `autoLoadRelations`, `listRule`, `viewRule`, `createRule`, `updateRule`, `deleteRule`, `created`, `updated`)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        ON CONFLICT(`id`) DO UPDATE SET
            `name` = excluded.`name`,
            `type` = excluded.`type`,
            `schema` = excluded.`schema`,
            `realtime` = excluded.`realtime`,
            `autoLoadRelations` = excluded.`autoLoadRelations`,
            `listRule` = excluded.`listRule`,
            `viewRule` = excluded.`viewRule`,
            `createRule` = excluded.`createRule`,
            `updateRule` = excluded.`updateRule`,
            `deleteRule` = excluded.`deleteRule`,
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
            Val::from(col.list_rule.clone()),
            Val::from(col.view_rule.clone()),
            Val::from(col.create_rule.clone()),
            Val::from(col.update_rule.clone()),
            Val::from(col.delete_rule.clone()),
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

    Collection {
        id: json_get_str(r, "id"),
        name: json_get_str(r, "name"),
        collection_type,
        schema,
        realtime,
        auto_load_relations,
        list_rule: json_get_opt_str(r, "listRule"),
        view_rule: json_get_opt_str(r, "viewRule"),
        create_rule: json_get_opt_str(r, "createRule"),
        update_rule: json_get_opt_str(r, "updateRule"),
        delete_rule: json_get_opt_str(r, "deleteRule"),
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
            SELECT `id`, `name`, `type`, `schema`, `realtime`, `autoLoadRelations`, `listRule`, `viewRule`, `createRule`, `updateRule`, `deleteRule`, `created`, `updated`
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
    let rows = pool
        .fetch_all(
            r#"
            SELECT `id`, `name`, `type`, `schema`, `realtime`, `autoLoadRelations`, `listRule`, `viewRule`, `createRule`, `updateRule`, `deleteRule`, `created`, `updated`
            FROM `_collections`
            ORDER BY `name` ASC
            "#,
            &[],
        )
        .await?;

    let list: Vec<Collection> = rows.iter().map(value_to_collection).collect();
    // Cache all collections in tenant memory
    for col in &list {
        pool.set_cached_collection(col);
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

    // 2. Convention Fallback (e.g. siswa_id -> siswa / siswas)
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
            list_rule: Some("".to_string()),
            view_rule: Some("".to_string()),
            create_rule: Some("@request.auth.id != \"\"".to_string()),
            update_rule: Some("@request.auth.id != \"\"".to_string()),
            delete_rule: Some("@request.auth.id != \"\"".to_string()),
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

pub async fn create_or_update_collection_table(pool: &DbPool, col: &Collection) -> Result<()> {
    // 1. Create table if not exists with base columns
    let mut columns_sql = vec![
        "`id` VARCHAR(255) PRIMARY KEY NOT NULL".to_string(),
        "`created` VARCHAR(64) NOT NULL".to_string(),
        "`updated` VARCHAR(64) NOT NULL".to_string(),
    ];

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

    for field in &col.schema {
        if columns_sql.iter().any(|c| c.starts_with(&format!("`{}` ", field.name))) {
            continue;
        }
        let col_type = sql_type_for_field(&field.field_type, field.unique);
        let nullability = if field.required { "NOT NULL" } else { "" };
        let unique = if field.unique { "UNIQUE" } else { "" };
        columns_sql.push(
            format!("`{}` {} {} {}", field.name, col_type, nullability, unique)
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" "),
        );
    }

    let create_sql = format!(
        "CREATE TABLE IF NOT EXISTS `{}` ({});",
        col.name,
        columns_sql.join(", ")
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
    }

    for field in &col.schema {
        if !has_col(&field.name) {
            let col_type = sql_type_for_field(&field.field_type, field.unique);
            let alter_sql = format!(
                "ALTER TABLE `{}` ADD COLUMN `{}` {};",
                col.name, field.name, col_type
            );
            let _ = pool.execute(&alter_sql, &[]).await;
        }
    }

    pool.invalidate_schema_cache(&col.name);
    pool.invalidate_schema_cache(&col.id);

    Ok(())
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
}
