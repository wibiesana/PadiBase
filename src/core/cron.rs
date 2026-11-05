use anyhow::{anyhow, Result};
use chrono::{DateTime, Datelike, Timelike, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicI64, Ordering};
use std::time::Instant;
use tracing::{error, info, warn};

use crate::core::db::{DbPool, Val, json_get_str};
use crate::core::env::Env;

/// Representation of a scheduled background task in PadiBase.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CronJob {
    pub id: String,
    pub name: String,
    pub schedule: String,
    pub job_type: String, // "sql" | "http"
    pub enabled: bool,

    // SQL options
    #[serde(default)]
    pub sql_query: Option<String>,

    // HTTP options
    #[serde(default)]
    pub http_method: Option<String>, // "GET", "POST", "PUT", "DELETE", "PATCH"
    #[serde(default)]
    pub http_url: Option<String>,
    #[serde(default)]
    pub http_headers: Option<HashMap<String, String>>,
    #[serde(default)]
    pub http_body: Option<String>,

    // Execution statistics
    #[serde(default)]
    pub last_run_at: Option<String>,
    #[serde(default)]
    pub last_status: Option<String>, // "success" | "failed" | "running"
    #[serde(default)]
    pub last_duration_ms: Option<u64>,
    #[serde(default)]
    pub last_error: Option<String>,
    #[serde(default)]
    pub last_result: Option<String>,

    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CronJobSummary {
    #[serde(flatten)]
    pub job: CronJob,
    pub next_run_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CronExecutionResult {
    pub job_id: String,
    pub status: String,
    pub duration_ms: u64,
    pub result: Option<String>,
    pub error: Option<String>,
}

// ================= CRON EXPRESSION PARSER & EVALUATOR =================

/// Resolve predefined cron aliases into standard 5-part expressions.
pub fn normalize_cron_alias(schedule: &str) -> String {
    let s = schedule.trim().to_lowercase();
    match s.as_str() {
        "@yearly" | "@annually" => "0 0 1 1 *".to_string(),
        "@monthly" => "0 0 1 * *".to_string(),
        "@weekly" => "0 0 * * 0".to_string(),
        "@daily" | "@midnight" => "0 0 * * *".to_string(),
        "@hourly" => "0 * * * *".to_string(),
        _ => {
            // Support "@every 5m", "@every 15m", "@every 1h", "@every 2h", "@every 1d"
            if let Some(rest) = s.strip_prefix("@every ") {
                let rest = rest.trim();
                if let Some(mins) = rest.strip_suffix('m') {
                    if let Ok(m) = mins.trim().parse::<u32>() {
                        if m >= 1 && m <= 59 {
                            return format!("*/{} * * * *", m);
                        }
                    }
                } else if let Some(hours) = rest.strip_suffix('h') {
                    if let Ok(h) = hours.trim().parse::<u32>() {
                        if h >= 1 && h <= 23 {
                            return format!("0 */{} * * *", h);
                        }
                    }
                } else if let Some(days) = rest.strip_suffix('d') {
                    if let Ok(d) = days.trim().parse::<u32>() {
                        if d >= 1 && d <= 30 {
                            return format!("0 0 */{} * *", d);
                        }
                    }
                }
            }
            schedule.trim().to_string()
        }
    }
}

/// Helper function to test if an integer value matches a cron field specification.
fn field_matches(spec: &str, val: u32, min: u32, max: u32) -> bool {
    let spec = spec.trim();
    if spec == "*" {
        return true;
    }

    // Split on comma for lists, e.g. "1,15,30"
    for part in spec.split(',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }

        // Step notation: "*/5" or "10-30/5"
        if let Some((range_part, step_part)) = part.split_once('/') {
            let step: u32 = match step_part.parse() {
                Ok(s) if s > 0 => s,
                _ => continue,
            };

            let (start, end) = if range_part == "*" {
                (min, max)
            } else if let Some((s_str, e_str)) = range_part.split_once('-') {
                match (s_str.parse::<u32>(), e_str.parse::<u32>()) {
                    (Ok(s), Ok(e)) => (s, e),
                    _ => continue,
                }
            } else {
                match range_part.parse::<u32>() {
                    Ok(s) => (s, max),
                    _ => continue,
                }
            };

            if val >= start && val <= end && (val - start) % step == 0 {
                return true;
            }
        }
        // Range notation: "1-5"
        else if let Some((s_str, e_str)) = part.split_once('-') {
            if let (Ok(s), Ok(e)) = (s_str.parse::<u32>(), e_str.parse::<u32>()) {
                if val >= s && val <= e {
                    return true;
                }
            }
        }
        // Exact number: "5"
        else if let Ok(n) = part.parse::<u32>() {
            if val == n {
                return true;
            }
        }
    }

    false
}

/// Check if a cron expression matches the given datetime (at minute precision).
pub fn is_cron_due(schedule: &str, dt: &DateTime<Utc>) -> bool {
    let normalized = normalize_cron_alias(schedule);
    let parts: Vec<&str> = normalized.split_whitespace().collect();
    if parts.len() != 5 {
        return false;
    }

    let min = dt.minute();
    let hour = dt.hour();
    let day = dt.day();
    let month = dt.month();
    // In chrono, weekday().num_days_from_sunday(): Sun=0, Mon=1, ..., Sat=6
    let weekday = dt.weekday().num_days_from_sunday();

    let min_matches = field_matches(parts[0], min, 0, 59);
    let hour_matches = field_matches(parts[1], hour, 0, 23);
    let day_matches = field_matches(parts[2], day, 1, 31);
    let month_matches = field_matches(parts[3], month, 1, 12);
    // Standard cron allows 0 or 7 for Sunday
    let dow_matches = field_matches(parts[4], weekday, 0, 6)
        || (weekday == 0 && field_matches(parts[4], 7, 0, 7));

    min_matches && hour_matches && day_matches && month_matches && dow_matches
}

/// Validate that a cron schedule expression is properly formatted.
pub fn validate_cron_schedule(schedule: &str) -> Result<String> {
    let normalized = normalize_cron_alias(schedule);
    let parts: Vec<&str> = normalized.split_whitespace().collect();
    if parts.len() != 5 {
        return Err(anyhow!(
            "Invalid cron format: must have 5 fields (minute hour day month day-of-week), got {}",
            parts.len()
        ));
    }

    // Test a dummy datetime
    let test_dt = Utc::now();
    let _ = is_cron_due(&normalized, &test_dt);

    Ok(normalized)
}

/// Calculate the next run time for a given cron schedule starting from `from`.
pub fn next_cron_run(schedule: &str, from: DateTime<Utc>) -> Option<DateTime<Utc>> {
    let normalized = normalize_cron_alias(schedule);
    let parts: Vec<&str> = normalized.split_whitespace().collect();
    if parts.len() != 5 {
        return None;
    }

    // Advance to next full minute
    let mut current = from + chrono::Duration::minutes(1);
    current = current
        .with_second(0)?
        .with_nanosecond(0)?;

    // Scan up to 525,600 minutes (1 year)
    for _ in 0..525_600 {
        if is_cron_due(&normalized, &current) {
            return Some(current);
        }
        current = current + chrono::Duration::minutes(1);
    }

    None
}

// ================= STORAGE & MANAGEMENT (IN _params) =================

/// Fetch all cron jobs from database settings.
pub async fn get_cron_jobs(pool: &DbPool) -> Result<Vec<CronJob>> {
    let row = pool
        .fetch_optional("SELECT `value` FROM `_params` WHERE `key` = 'cron_jobs'", &[])
        .await?;

    if let Some(r) = row {
        let val_str = json_get_str(&r, "value");
        if let Ok(jobs) = serde_json::from_str::<Vec<CronJob>>(&val_str) {
            return Ok(jobs);
        }
    }

    Ok(vec![])
}

/// Save all cron jobs into database settings.
pub async fn save_cron_jobs(pool: &DbPool, jobs: &[CronJob]) -> Result<()> {
    let json_str = serde_json::to_string(jobs)?;
    let sql = if pool.is_mysql() {
        "INSERT INTO `_params` (`key`, `value`) VALUES ('cron_jobs', ?) ON DUPLICATE KEY UPDATE `value` = VALUES(`value`)"
    } else {
        "INSERT INTO `_params` (`key`, `value`) VALUES ('cron_jobs', ?) ON CONFLICT(`key`) DO UPDATE SET `value` = excluded.`value`"
    };

    pool.execute(sql, &[Val::from(&json_str)]).await?;
    Ok(())
}

/// Add or update a specific cron job.
pub async fn upsert_cron_job(pool: &DbPool, job: CronJob) -> Result<CronJob> {
    let mut jobs = get_cron_jobs(pool).await?;
    let mut found = false;

    for item in jobs.iter_mut() {
        if item.id == job.id {
            *item = job.clone();
            found = true;
            break;
        }
    }

    if !found {
        jobs.push(job.clone());
    }

    save_cron_jobs(pool, &jobs).await?;
    Ok(job)
}

/// Delete a cron job by ID.
pub async fn delete_cron_job(pool: &DbPool, id: &str) -> Result<bool> {
    let mut jobs = get_cron_jobs(pool).await?;
    let initial_len = jobs.len();
    jobs.retain(|j| j.id != id);

    if jobs.len() != initial_len {
        save_cron_jobs(pool, &jobs).await?;
        Ok(true)
    } else {
        Ok(false)
    }
}

// ================= EXECUTION ENGINE =================

/// Execute a cron job immediately (either SQL query or HTTP API call).
pub async fn execute_cron_job(pool: &DbPool, job_id: &str) -> Result<CronExecutionResult> {
    let mut jobs = get_cron_jobs(pool).await?;
    let job_idx = jobs
        .iter()
        .position(|j| j.id == job_id)
        .ok_or_else(|| anyhow!("Cron job with ID '{}' not found", job_id))?;

    let job = jobs[job_idx].clone();
    let start_time = Instant::now();
    let run_at = Utc::now().to_rfc3339();

    let mut status = "success".to_string();
    let mut result_msg: Option<String> = None;
    let mut error_msg: Option<String> = None;

    match job.job_type.to_lowercase().as_str() {
        "sql" => {
            let query = job.sql_query.as_deref().unwrap_or("").trim();
            if query.is_empty() {
                status = "failed".to_string();
                error_msg = Some("SQL query is empty".to_string());
            } else {
                // Split multi-statement SQL by semicolon while ignoring comments
                let statements: Vec<&str> = query
                    .split(';')
                    .map(|s| s.trim())
                    .filter(|s| !s.is_empty())
                    .collect();

                let mut total_affected: u64 = 0;
                let mut exec_err: Option<String> = None;

                for stmt in &statements {
                    match pool.execute(stmt, &[]).await {
                        Ok((affected, _)) => {
                            total_affected += affected;
                        }
                        Err(e) => {
                            exec_err = Some(format!("Statement failed: '{}' -> {}", stmt, e));
                            break;
                        }
                    }
                }

                if let Some(err) = exec_err {
                    status = "failed".to_string();
                    error_msg = Some(err);
                } else {
                    result_msg = Some(format!(
                        "Executed {} SQL statement(s) successfully ({} rows affected)",
                        statements.len(),
                        total_affected
                    ));
                }
            }
        }
        "http" => {
            let raw_url = job.http_url.as_deref().unwrap_or("").trim();
            if raw_url.is_empty() {
                status = "failed".to_string();
                error_msg = Some("HTTP URL is empty".to_string());
            } else {
                // Support relative URL (e.g. "/api/...") by resolving to local app URL
                let final_url = if raw_url.starts_with('/') {
                    let host = Env::get_or("APP_HOST", "127.0.0.1");
                    let port = Env::get_or("APP_PORT", "8085");
                    format!("http://{}:{}{}", host, port, raw_url)
                } else {
                    raw_url.to_string()
                };

                let method_str = job.http_method.as_deref().unwrap_or("GET").to_uppercase();
                let client = reqwest::Client::builder()
                    .timeout(std::time::Duration::from_secs(60))
                    .redirect(reqwest::redirect::Policy::limited(3))
                    .build();

                match client {
                    Ok(client) => {
                        let method = match method_str.as_str() {
                            "POST" => reqwest::Method::POST,
                            "PUT" => reqwest::Method::PUT,
                            "DELETE" => reqwest::Method::DELETE,
                            "PATCH" => reqwest::Method::PATCH,
                            "HEAD" => reqwest::Method::HEAD,
                            _ => reqwest::Method::GET,
                        };

                        let mut req_builder = client.request(method, &final_url);
                        req_builder = req_builder.header("User-Agent", "PadiBase-Cron/1.0");

                        if let Some(headers) = &job.http_headers {
                            for (k, v) in headers {
                                req_builder = req_builder.header(k, v);
                            }
                        }

                        if let Some(body) = &job.http_body {
                            if !body.is_empty() && (method_str == "POST" || method_str == "PUT" || method_str == "PATCH") {
                                if !job.http_headers.as_ref().map_or(false, |h| h.keys().any(|k| k.eq_ignore_ascii_case("content-type"))) {
                                    req_builder = req_builder.header("Content-Type", "application/json");
                                }
                                req_builder = req_builder.body(body.clone());
                            }
                        }

                        match req_builder.send().await {
                            Ok(mut resp) => {
                                let status_code = resp.status();
                                // Read at most 8 KiB so a huge/endless response can't exhaust memory.
                                const MAX_PREVIEW_BYTES: usize = 8 * 1024;
                                let mut buf: Vec<u8> = Vec::new();
                                while buf.len() < MAX_PREVIEW_BYTES {
                                    match resp.chunk().await {
                                        Ok(Some(chunk)) => buf.extend_from_slice(&chunk),
                                        _ => break,
                                    }
                                }
                                let resp_text = String::from_utf8_lossy(&buf).into_owned();
                                // Truncate on a char boundary (byte slicing can panic on UTF-8).
                                let preview = if resp_text.chars().count() > 500 {
                                    let cut: String = resp_text.chars().take(500).collect();
                                    format!("{}... (truncated)", cut)
                                } else {
                                    resp_text
                                };

                                if status_code.is_success() {
                                    result_msg = Some(format!("HTTP {} OK: {}", status_code.as_u16(), preview));
                                } else {
                                    status = "failed".to_string();
                                    error_msg = Some(format!("HTTP Error {}: {}", status_code.as_u16(), preview));
                                }
                            }
                            Err(e) => {
                                status = "failed".to_string();
                                error_msg = Some(format!("HTTP request failed: {}", e));
                            }
                        }
                    }
                    Err(e) => {
                        status = "failed".to_string();
                        error_msg = Some(format!("Failed to build HTTP client: {}", e));
                    }
                }
            }
        }
        other => {
            status = "failed".to_string();
            error_msg = Some(format!("Unsupported job type: {}", other));
        }
    }

    let duration_ms = start_time.elapsed().as_millis() as u64;

    // Update job runtime metadata
    jobs[job_idx].last_run_at = Some(run_at);
    jobs[job_idx].last_status = Some(status.clone());
    jobs[job_idx].last_duration_ms = Some(duration_ms);
    jobs[job_idx].last_result = result_msg.clone();
    jobs[job_idx].last_error = error_msg.clone();
    jobs[job_idx].updated_at = Utc::now().to_rfc3339();

    // Persist changes
    let _ = save_cron_jobs(pool, &jobs).await;

    Ok(CronExecutionResult {
        job_id: job.id,
        status,
        duration_ms,
        result: result_msg,
        error: error_msg,
    })
}

// ================= BACKGROUND SCHEDULER =================

static LAST_RUN_MINUTE: AtomicI64 = AtomicI64::new(-1);

/// Start the background cron scheduler loop.
pub fn start_cron_scheduler(pool: DbPool) {
    tokio::spawn(async move {
        info!("Starting PadiBase Cron background scheduler...");
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(15));

        loop {
            interval.tick().await;

            let now = Utc::now();
            let current_minute = now.timestamp() / 60;

            // Only evaluate once per minute
            let last = LAST_RUN_MINUTE.load(Ordering::Relaxed);
            if current_minute == last {
                continue;
            }

            LAST_RUN_MINUTE.store(current_minute, Ordering::Relaxed);

            // Fetch active jobs
            if let Ok(jobs) = get_cron_jobs(&pool).await {
                for job in jobs {
                    if !job.enabled {
                        continue;
                    }

                    if is_cron_due(&job.schedule, &now) {
                        let pool_clone = pool.clone();
                        let j_id = job.id.clone();
                        let j_name = job.name.clone();

                        tokio::spawn(async move {
                            info!("Cron scheduler: Running job '{}' (ID: {})", j_name, j_id);
                            match execute_cron_job(&pool_clone, &j_id).await {
                                Ok(res) => {
                                    if res.status == "success" {
                                        info!("Cron job '{}' completed in {}ms", j_name, res.duration_ms);
                                    } else {
                                        warn!("Cron job '{}' failed in {}ms: {:?}", j_name, res.duration_ms, res.error);
                                    }
                                }
                                Err(e) => {
                                    error!("Cron job '{}' execution error: {}", j_name, e);
                                }
                            }
                        });
                    }
                }
            }
        }
    });
}
