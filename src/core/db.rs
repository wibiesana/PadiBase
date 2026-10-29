//! Multi-Database Engine for padiBase (SQLite, MySQL, MariaDB, PostgreSQL)
//! Inspired by and compatible with Padi Rust Api Framework (`padi_core::database`).

use anyhow::{anyhow, Result};
use serde_json::{json, Map, Value};
use sqlx::{
    mysql::MySqlPoolOptions,
    postgres::PgPoolOptions,
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous},
    Column, Row,
};
use std::path::Path;
use std::str::FromStr;
use tracing::info;

use super::env::Env;
use super::schema::{Collection, CollectionType, FieldType};

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

/// Unified multi-database connection pool (SQLite / MySQL / MariaDB / PostgreSQL).
#[derive(Clone, Debug)]
pub enum DbPool {
    Sqlite(sqlx::SqlitePool),
    MySql(sqlx::MySqlPool, DbDriver),
    Postgres(sqlx::PgPool),
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
    pub fn driver(&self) -> DbDriver {
        match self {
            DbPool::Sqlite(_) => DbDriver::Sqlite,
            DbPool::MySql(_, d) => *d,
            DbPool::Postgres(_) => DbDriver::Postgres,
        }
    }

    pub fn driver_name(&self) -> &'static str {
        self.driver().as_str()
    }

    pub fn is_sqlite(&self) -> bool {
        matches!(self, DbPool::Sqlite(_))
    }

    pub fn is_mysql(&self) -> bool {
        matches!(self, DbPool::MySql(_, _))
    }

    pub fn is_postgres(&self) -> bool {
        matches!(self, DbPool::Postgres(_))
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
        let res = match self {
            DbPool::Sqlite(p) => {
                let rows = bind_sqlite(&prepared, params).fetch_all(p).await?;
                rows.iter().map(sqlite_row_to_json).collect()
            }
            DbPool::MySql(p, _) => {
                let rows = bind_mysql(&prepared, params).fetch_all(p).await?;
                rows.iter().map(mysql_row_to_json).collect()
            }
            DbPool::Postgres(p) => {
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
        let res = match self {
            DbPool::Sqlite(p) => {
                let r = bind_sqlite(&prepared, params).execute(p).await?;
                (r.rows_affected(), Some(r.last_insert_rowid()))
            }
            DbPool::MySql(p, _) => {
                let r = bind_mysql(&prepared, params).execute(p).await?;
                (r.rows_affected(), Some(r.last_insert_id() as i64))
            }
            DbPool::Postgres(p) => {
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
            DbPool::Sqlite(p)
        }
        DbDriver::MySql | DbDriver::MariaDb => {
            let p = MySqlPoolOptions::new()
                .max_connections(max_conn)
                .connect(&db_url)
                .await?;
            info!("Connected to {} database", driver.as_str().to_uppercase());
            DbPool::MySql(p, driver)
        }
        DbDriver::Postgres => {
            let p = PgPoolOptions::new()
                .max_connections(max_conn)
                .connect(&db_url)
                .await?;
            info!("Connected to PostgreSQL database");
            DbPool::Postgres(p)
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

    // Ensure `realtime` column exists on existing `_collections` tables
    if let Ok(cols) = get_existing_table_columns(pool, "_collections").await {
        if !cols.iter().any(|c| c.eq_ignore_ascii_case("realtime")) {
            let _ = pool
                .execute(
                    "ALTER TABLE `_collections` ADD COLUMN `realtime` INTEGER DEFAULT 1;",
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

    let sql = if pool.is_mysql() {
        r#"
        INSERT INTO `_collections` (`id`, `name`, `type`, `schema`, `realtime`, `listRule`, `viewRule`, `createRule`, `updateRule`, `deleteRule`, `created`, `updated`)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        ON DUPLICATE KEY UPDATE
            `name` = VALUES(`name`),
            `type` = VALUES(`type`),
            `schema` = VALUES(`schema`),
            `realtime` = VALUES(`realtime`),
            `listRule` = VALUES(`listRule`),
            `viewRule` = VALUES(`viewRule`),
            `createRule` = VALUES(`createRule`),
            `updateRule` = VALUES(`updateRule`),
            `deleteRule` = VALUES(`deleteRule`),
            `updated` = VALUES(`updated`)
        "#
    } else {
        r#"
        INSERT INTO `_collections` (`id`, `name`, `type`, `schema`, `realtime`, `listRule`, `viewRule`, `createRule`, `updateRule`, `deleteRule`, `created`, `updated`)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        ON CONFLICT(`id`) DO UPDATE SET
            `name` = excluded.`name`,
            `type` = excluded.`type`,
            `schema` = excluded.`schema`,
            `realtime` = excluded.`realtime`,
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

    Collection {
        id: json_get_str(r, "id"),
        name: json_get_str(r, "name"),
        collection_type,
        schema,
        realtime,
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
    let row = pool
        .fetch_optional(
            r#"
            SELECT `id`, `name`, `type`, `schema`, `realtime`, `listRule`, `viewRule`, `createRule`, `updateRule`, `deleteRule`, `created`, `updated`
            FROM `_collections`
            WHERE `id` = ? OR `name` = ?
            "#,
            &[Val::from(name_or_id), Val::from(name_or_id)],
        )
        .await?;

    Ok(row.as_ref().map(value_to_collection))
}

pub async fn list_collections(pool: &DbPool) -> Result<Vec<Collection>> {
    let rows = pool
        .fetch_all(
            r#"
            SELECT `id`, `name`, `type`, `schema`, `realtime`, `listRule`, `viewRule`, `createRule`, `updateRule`, `deleteRule`, `created`, `updated`
            FROM `_collections`
            ORDER BY `name` ASC
            "#,
            &[],
        )
        .await?;

    Ok(rows.iter().map(value_to_collection).collect())
}

pub async fn delete_collection(pool: &DbPool, name_or_id: &str) -> Result<bool> {
    let col = match get_collection_by_name_or_id(pool, name_or_id).await? {
        Some(c) => c,
        None => return Ok(false),
    };

    let sql = format!("DROP TABLE IF EXISTS `{}`;", col.name);
    pool.execute(&sql, &[]).await?;

    pool.execute(
        "DELETE FROM `_collections` WHERE `id` = ?",
        &[Val::from(&col.id)],
    )
    .await?;

    Ok(true)
}

/// Introspect existing column names for a table across SQLite, PostgreSQL, and MySQL/MariaDB.
pub async fn get_existing_table_columns(pool: &DbPool, table_name: &str) -> Result<Vec<String>> {
    if pool.is_sqlite() {
        let pragma_sql = format!("PRAGMA table_info(`{}`);", table_name);
        let rows = pool.fetch_all(&pragma_sql, &[]).await?;
        Ok(rows
            .into_iter()
            .map(|r| json_get_str(&r, "name"))
            .filter(|s| !s.is_empty())
            .collect())
    } else if pool.is_postgres() {
        let rows = pool
            .fetch_all(
                "SELECT column_name FROM information_schema.columns WHERE table_schema = 'public' AND table_name = ?",
                &[Val::from(table_name)],
            )
            .await?;
        Ok(rows
            .into_iter()
            .map(|r| json_get_str(&r, "column_name"))
            .filter(|s| !s.is_empty())
            .collect())
    } else {
        // MySQL / MariaDB
        let rows = pool
            .fetch_all(
                "SELECT column_name FROM information_schema.columns WHERE table_schema = DATABASE() AND table_name = ?",
                &[Val::from(table_name)],
            )
            .await?;
        Ok(rows
            .into_iter()
            .map(|r| json_get_str(&r, "column_name"))
            .filter(|s| !s.is_empty())
            .collect())
    }
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
