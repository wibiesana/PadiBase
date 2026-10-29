//! Env - Environment Variable Loader for padiBase (ported from Padi Rust Api Framework)
//!
//! Loads `.env` files once and caches values in an in-memory cache.

use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{LazyLock, RwLock};

static CACHE: LazyLock<RwLock<HashMap<String, String>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

static LOADED: AtomicBool = AtomicBool::new(false);

pub struct Env;

impl Env {
    /// Load `.env` file once.
    pub fn load(path: impl AsRef<Path>) -> bool {
        Self::load_with(path, false)
    }

    /// Load `.env` file with optional forced reloading.
    pub fn load_with(path: impl AsRef<Path>, force_reload: bool) -> bool {
        let path = path.as_ref();

        if (LOADED.load(Ordering::SeqCst) && !force_reload) || !path.exists() {
            return false;
        }

        let Ok(content) = fs::read_to_string(path) else {
            return false;
        };

        LOADED.store(true, Ordering::SeqCst);

        let mut cache = CACHE.write().unwrap_or_else(|poisoned| poisoned.into_inner());

        for line in content.lines() {
            if let Some((name, value)) = Self::parse_line(line) {
                let already_cached = cache.contains_key(&name);
                let already_in_process = std::env::var(&name).is_ok();

                if force_reload || (!already_cached && !already_in_process) {
                    unsafe {
                        std::env::set_var(&name, &value);
                    }
                    cache.insert(name, value);
                } else if !already_cached && already_in_process {
                    if let Ok(process_val) = std::env::var(&name) {
                        cache.insert(name, process_val);
                    }
                }
            }
        }

        true
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

    /// Get environment variable value.
    pub fn get(key: &str) -> Option<String> {
        if let Ok(cache) = CACHE.read() {
            if let Some(val) = cache.get(key) {
                if !val.is_empty() {
                    return Some(val.clone());
                }
            }
        }

        if let Ok(val) = std::env::var(key) {
            if !val.is_empty() {
                if let Ok(mut cache) = CACHE.write() {
                    cache.insert(key.to_string(), val.clone());
                }
                return Some(val);
            }
        }

        None
    }

    /// Get environment variable or return default.
    pub fn get_or(key: &str, default: &str) -> String {
        Self::get(key).unwrap_or_else(|| default.to_string())
    }

    /// Get environment variable parsed as `i64` or return default.
    pub fn get_int(key: &str, default: i64) -> i64 {
        Self::get(key)
            .and_then(|v| v.trim().parse::<i64>().ok())
            .unwrap_or(default)
    }

    /// Get environment variable parsed as boolean or return default.
    pub fn get_bool(key: &str, default: bool) -> bool {
        match Self::get(key) {
            Some(v) => {
                let s = v.trim().to_ascii_lowercase();
                s == "true" || s == "1" || s == "yes" || s == "on"
            }
            None => default,
        }
    }

    /// Check if variable is true (`true`, `1`, `yes`, `on`).
    pub fn is_true(key: &str) -> bool {
        Self::get_bool(key, false)
    }

    /// True when running in production mode (`APP_ENV=production` or `APP_ENV=prod`).
    pub fn is_production() -> bool {
        let env = Self::get_or("APP_ENV", "development").to_ascii_lowercase();
        env == "production" || env == "prod"
    }

    /// True when running in development mode (`APP_ENV=development`, `dev`, `local`).
    pub fn is_development() -> bool {
        let env = Self::get_or("APP_ENV", "development").to_ascii_lowercase();
        env == "development" || env == "dev" || env == "local"
    }

    /// True when `APP_DEBUG=true` (or default true in development).
    pub fn is_debug() -> bool {
        Self::get_bool("APP_DEBUG", Self::is_development())
    }

    /// Set an environment variable in both cache and process environment.
    pub fn set(key: &str, value: &str) {
        if let Ok(mut cache) = CACHE.write() {
            cache.insert(key.to_string(), value.to_string());
        }
        unsafe {
            std::env::set_var(key, value);
        }
    }
}
