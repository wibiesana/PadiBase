//! Env - Ultra-Fast Single-Read Environment & Configuration Cache for padiBase
//!
//! Reads `.env` and process environment variables ONCE at startup and caches
//! both raw key-values and pre-parsed hot-path atomic flags in RAM (zero disk I/O,
//! zero syscalls, and lock-free atomic reads during HTTP requests and SQL queries).

use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::{LazyLock, RwLock};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum ResponseFormat {
    Full = 0,
    Simple = 1,
    Raw = 2,
}

impl ResponseFormat {
    #[inline]
    pub fn from_u8(v: u8) -> Self {
        match v {
            1 => ResponseFormat::Simple,
            2 => ResponseFormat::Raw,
            _ => ResponseFormat::Full,
        }
    }

    #[inline]
    pub fn as_str(&self) -> &'static str {
        match self {
            ResponseFormat::Full => "full",
            ResponseFormat::Simple => "simple",
            ResponseFormat::Raw => "raw",
        }
    }
}

#[derive(Clone, Debug)]
pub struct AppConfig {
    pub app_name: String,
    pub app_env: String,
    pub app_host: String,
    pub app_port: String,
    pub jwt_secret: String,
    pub cors_allowed_origins: String,
    pub db_max_connections: u32,
    pub is_production: bool,
    pub is_development: bool,
    pub is_debug: bool,
    pub auto_load_relations: bool,
    pub response_format: ResponseFormat,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            app_name: "PadiBase Studio".to_string(),
            app_env: "development".to_string(),
            app_host: "127.0.0.1".to_string(),
            app_port: "8085".to_string(),
            jwt_secret: "PadiBase_default_secure_secret_key_change_me_in_prod".to_string(),
            cors_allowed_origins: "*".to_string(),
            db_max_connections: 20,
            is_production: false,
            is_development: true,
            is_debug: true,
            auto_load_relations: false,
            response_format: ResponseFormat::Full,
        }
    }
}

static CACHE: LazyLock<RwLock<HashMap<String, String>>> =
    LazyLock::new(|| RwLock::new(HashMap::with_capacity(64)));

static APP_CONFIG: LazyLock<RwLock<AppConfig>> =
    LazyLock::new(|| RwLock::new(AppConfig::default()));

static LOADED: AtomicBool = AtomicBool::new(false);

// Lock-free hot-path atomic flags (updated once on load or when Env::set is called)
static FAST_IS_PRODUCTION: AtomicBool = AtomicBool::new(false);
static FAST_IS_DEVELOPMENT: AtomicBool = AtomicBool::new(true);
static FAST_IS_DEBUG: AtomicBool = AtomicBool::new(true);
static FAST_AUTO_LOAD_RELATIONS: AtomicBool = AtomicBool::new(false);
static FAST_RESPONSE_FORMAT: AtomicU8 = AtomicU8::new(ResponseFormat::Full as u8);

fn parse_bool_str(v: &str) -> bool {
    let s = v.trim();
    s.eq_ignore_ascii_case("true")
        || s == "1"
        || s.eq_ignore_ascii_case("yes")
        || s.eq_ignore_ascii_case("on")
}

fn map_get_non_empty<'a>(map: &'a HashMap<String, String>, key: &str) -> Option<&'a str> {
    map.get(key).map(|s| s.as_str()).filter(|s| !s.is_empty())
}

fn refresh_derived_config(cache: &HashMap<String, String>) {
    let app_env = map_get_non_empty(cache, "APP_ENV")
        .unwrap_or("development")
        .trim()
        .to_string();
    let env_lower = app_env.to_ascii_lowercase();

    let is_prod = env_lower == "production" || env_lower == "prod";
    let is_dev = env_lower == "development" || env_lower == "dev" || env_lower == "local";
    let is_debug = map_get_non_empty(cache, "APP_DEBUG")
        .map(parse_bool_str)
        .unwrap_or(is_dev);
    let auto_load = map_get_non_empty(cache, "AUTO_LOAD_RELATIONS")
        .map(parse_bool_str)
        .unwrap_or(false);

    let resp_fmt = match map_get_non_empty(cache, "RESPONSE_FORMAT") {
        Some(f) if f.trim().eq_ignore_ascii_case("simple") => ResponseFormat::Simple,
        Some(f) if f.trim().eq_ignore_ascii_case("raw") => ResponseFormat::Raw,
        _ => ResponseFormat::Full,
    };

    FAST_IS_PRODUCTION.store(is_prod, Ordering::Release);
    FAST_IS_DEVELOPMENT.store(is_dev, Ordering::Release);
    FAST_IS_DEBUG.store(is_debug, Ordering::Release);
    FAST_AUTO_LOAD_RELATIONS.store(auto_load, Ordering::Release);
    FAST_RESPONSE_FORMAT.store(resp_fmt as u8, Ordering::Release);

    let app_name = map_get_non_empty(cache, "APP_NAME")
        .unwrap_or("PadiBase Studio")
        .to_string();
    let app_host = map_get_non_empty(cache, "APP_HOST")
        .unwrap_or("127.0.0.1")
        .to_string();
    let app_port = map_get_non_empty(cache, "APP_PORT")
        .unwrap_or("8085")
        .to_string();
    let jwt_secret = map_get_non_empty(cache, "JWT_SECRET")
        .unwrap_or("PadiBase_default_secure_secret_key_change_me_in_prod")
        .to_string();
    let cors_allowed_origins = map_get_non_empty(cache, "CORS_ALLOWED_ORIGINS")
        .unwrap_or("*")
        .to_string();
    let db_max_connections = map_get_non_empty(cache, "DB_MAX_CONNECTIONS")
        .and_then(|v| v.trim().parse::<u32>().ok())
        .unwrap_or(20)
        .max(1);

    let mut cfg = APP_CONFIG
        .write()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    *cfg = AppConfig {
        app_name,
        app_env,
        app_host,
        app_port,
        jwt_secret,
        cors_allowed_origins,
        db_max_connections,
        is_production: is_prod,
        is_development: is_dev,
        is_debug,
        auto_load_relations: auto_load,
        response_format: resp_fmt,
    };
}

pub struct Env;

impl Env {
    /// Ensure `.env` and process environment are loaded once.
    #[inline]
    pub fn ensure_loaded() {
        if !LOADED.load(Ordering::Acquire) {
            Self::load(".env");
        }
    }

    /// Load `.env` file once. Subsequent calls are no-ops returning `false` with zero I/O.
    #[inline]
    pub fn load(path: impl AsRef<Path>) -> bool {
        Self::load_with(path, false)
    }

    /// Load `.env` file with optional forced reloading.
    pub fn load_with(path: impl AsRef<Path>, force_reload: bool) -> bool {
        if LOADED.load(Ordering::Acquire) && !force_reload {
            return false;
        }

        let path = path.as_ref();
        let file_content = fs::read_to_string(path).ok();

        let mut cache = CACHE
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner());

        // 1. Snapshot current process environment variables once into memory
        for (k, v) in std::env::vars() {
            if !v.is_empty() {
                cache.entry(k).or_insert(v);
            }
        }

        // 2. Parse `.env` file if present
        let loaded_from_file = if let Some(content) = file_content {
            for line in content.lines() {
                if let Some((name, value)) = Self::parse_line(line) {
                    let already_in_process = std::env::var_os(&name).is_some();
                    if force_reload || !already_in_process {
                        unsafe {
                            std::env::set_var(&name, &value);
                        }
                        cache.insert(name, value);
                    } else if let Ok(process_val) = std::env::var(&name) {
                        cache.insert(name, process_val);
                    }
                }
            }
            true
        } else {
            false
        };

        refresh_derived_config(&cache);
        LOADED.store(true, Ordering::Release);

        loaded_from_file
    }

    /// Parse a single line from a `.env` file.
    pub fn parse_line(line: &str) -> Option<(String, String)> {
        let mut trimmed = line.trim_start();

        if let Some(rest) = trimmed.strip_prefix("export ") {
            trimmed = rest.trim_start();
        }

        if trimmed.is_empty() || trimmed.starts_with('#') {
            return None;
        }

        let eq_pos = trimmed.find('=')?;
        let name = trimmed[..eq_pos].trim();
        if name.is_empty() {
            return None;
        }

        let mut value = trimmed[eq_pos + 1..].trim();

        if !value.is_empty() && !value.starts_with('"') && !value.starts_with('\'') {
            if let Some(hash_pos) = value.find(" #") {
                value = value[..hash_pos].trim_end();
            }
        }

        let clean_value = if value.len() >= 2 {
            let bytes = value.as_bytes();
            let first = bytes[0];
            let last = bytes[bytes.len() - 1];
            if (first == b'"' && last == b'"') || (first == b'\'' && last == b'\'') {
                &value[1..value.len() - 1]
            } else {
                value
            }
        } else {
            value
        };

        Some((name.to_string(), clean_value.to_string()))
    }

    /// Get environment variable value from in-memory cache with zero disk I/O and zero syscalls.
    #[inline]
    pub fn get(key: &str) -> Option<String> {
        Self::ensure_loaded();
        let cache = CACHE
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        cache.get(key).filter(|v| !v.is_empty()).cloned()
    }

    /// Get environment variable or return default.
    #[inline]
    pub fn get_or(key: &str, default: &str) -> String {
        Self::get(key).unwrap_or_else(|| default.to_string())
    }

    /// Get environment variable parsed as `i64` or return default.
    #[inline]
    pub fn get_int(key: &str, default: i64) -> i64 {
        Self::get(key)
            .and_then(|v| v.trim().parse::<i64>().ok())
            .unwrap_or(default)
    }

    /// Get environment variable parsed as boolean or return default.
    #[inline]
    pub fn get_bool(key: &str, default: bool) -> bool {
        match Self::get(key) {
            Some(v) => parse_bool_str(&v),
            None => default,
        }
    }

    /// Check if variable is true (`true`, `1`, `yes`, `on`).
    #[inline]
    pub fn is_true(key: &str) -> bool {
        Self::get_bool(key, false)
    }

    /// Lock-free O(1) check: True when running in production mode (`APP_ENV=production` or `prod`).
    #[inline]
    pub fn is_production() -> bool {
        Self::ensure_loaded();
        FAST_IS_PRODUCTION.load(Ordering::Relaxed)
    }

    /// Lock-free O(1) check: True when running in development mode (`APP_ENV=development`, `dev`, `local`).
    #[inline]
    pub fn is_development() -> bool {
        Self::ensure_loaded();
        FAST_IS_DEVELOPMENT.load(Ordering::Relaxed)
    }

    /// Lock-free O(1) check: True when `APP_DEBUG=true` (or default true in development).
    #[inline]
    pub fn is_debug() -> bool {
        Self::ensure_loaded();
        FAST_IS_DEBUG.load(Ordering::Relaxed)
    }

    /// Lock-free O(1) check: True when `AUTO_LOAD_RELATIONS=true`.
    #[inline]
    pub fn auto_load_relations() -> bool {
        Self::ensure_loaded();
        FAST_AUTO_LOAD_RELATIONS.load(Ordering::Relaxed)
    }

    /// Lock-free O(1) getter for `RESPONSE_FORMAT` (`Full`, `Simple`, `Raw`).
    #[inline]
    pub fn response_format() -> ResponseFormat {
        Self::ensure_loaded();
        ResponseFormat::from_u8(FAST_RESPONSE_FORMAT.load(Ordering::Relaxed))
    }

    /// Get the cached `AppConfig` snapshot.
    #[inline]
    pub fn config() -> AppConfig {
        Self::ensure_loaded();
        APP_CONFIG
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    }

    /// Set an environment variable in cache, process environment, and lock-free atomic flags.
    pub fn set(key: &str, value: &str) {
        Self::ensure_loaded();
        let mut cache = CACHE
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        cache.insert(key.to_string(), value.to_string());
        unsafe {
            std::env::set_var(key, value);
        }
        refresh_derived_config(&cache);
    }
}
