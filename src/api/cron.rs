use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    Json,
};
use chrono::Utc;
use serde_json::{json, Value};
use uuid::Uuid;

use crate::core::app::AppState;
use crate::core::auth::require_admin;
use crate::core::cron::{
    delete_cron_job, execute_cron_job, get_cron_jobs, next_cron_run, upsert_cron_job,
    validate_cron_schedule, CronJob, CronJobSummary,
};

/// GET /api/cron/jobs - List all cron jobs with their next run timestamp.
pub async fn list_jobs(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, (StatusCode, Json<Value>)> {
    require_admin(&state, &headers).await?;

    let jobs = get_cron_jobs(&state.db).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "code": 500, "message": e.to_string() })),
        )
    })?;

    let now = Utc::now();
    let summaries: Vec<CronJobSummary> = jobs
        .into_iter()
        .map(|job| {
            let next_run = if job.enabled {
                next_cron_run(&job.schedule, now).map(|dt| dt.to_rfc3339())
            } else {
                None
            };
            CronJobSummary {
                job,
                next_run_at: next_run,
            }
        })
        .collect();

    Ok(Json(json!({
        "code": 200,
        "message": "Cron jobs retrieved successfully",
        "items": summaries
    })))
}

/// POST /api/cron/jobs - Create a new cron job.
pub async fn create_job(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<Value>,
) -> Result<impl IntoResponse, (StatusCode, Json<Value>)> {
    require_admin(&state, &headers).await?;

    let name = payload
        .get("name")
        .and_then(|v| v.as_str())
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| {
            (
                StatusCode::BAD_REQUEST,
                Json(json!({ "code": 400, "message": "Job 'name' is required." })),
            )
        })?;

    let raw_schedule = payload
        .get("schedule")
        .and_then(|v| v.as_str())
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| {
            (
                StatusCode::BAD_REQUEST,
                Json(json!({ "code": 400, "message": "Job 'schedule' expression is required." })),
            )
        })?;

    let valid_schedule = validate_cron_schedule(raw_schedule).map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            Json(json!({ "code": 400, "message": format!("Invalid cron schedule: {}", e) })),
        )
    })?;

    let job_type = payload
        .get("job_type")
        .and_then(|v| v.as_str())
        .map(|s| s.trim().to_lowercase())
        .unwrap_or_else(|| "sql".to_string());

    if job_type != "sql" && job_type != "http" {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({ "code": 400, "message": "Job 'job_type' must be 'sql' or 'http'." })),
        ));
    }

    let enabled = payload
        .get("enabled")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);

    let sql_query = payload
        .get("sql_query")
        .and_then(|v| v.as_str())
        .map(|s| s.trim().to_string());

    let http_method = payload
        .get("http_method")
        .and_then(|v| v.as_str())
        .map(|s| s.trim().to_uppercase());

    let http_url = payload
        .get("http_url")
        .and_then(|v| v.as_str())
        .map(|s| s.trim().to_string());

    let http_headers = payload
        .get("http_headers")
        .and_then(|v| serde_json::from_value(v.clone()).ok());

    let http_body = payload
        .get("http_body")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    if job_type == "sql" && sql_query.as_ref().map_or(true, |q| q.is_empty()) {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({ "code": 400, "message": "Field 'sql_query' cannot be empty for SQL jobs." })),
        ));
    }

    if job_type == "http" && http_url.as_ref().map_or(true, |u| u.is_empty()) {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({ "code": 400, "message": "Field 'http_url' cannot be empty for HTTP jobs." })),
        ));
    }

    let now_str = Utc::now().to_rfc3339();
    let job = CronJob {
        id: Uuid::new_v4().to_string(),
        name: name.to_string(),
        schedule: valid_schedule,
        job_type,
        enabled,
        sql_query,
        http_method,
        http_url,
        http_headers,
        http_body,
        last_run_at: None,
        last_status: None,
        last_duration_ms: None,
        last_error: None,
        last_result: None,
        created_at: now_str.clone(),
        updated_at: now_str,
    };

    upsert_cron_job(&state.db, job.clone())
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "code": 500, "message": e.to_string() })),
            )
        })?;

    Ok((
        StatusCode::CREATED,
        Json(json!(job)),
    ))
}

/// PATCH /api/cron/jobs/:id - Update an existing cron job.
pub async fn update_job(
    State(state): State<AppState>,
    Path(id): Path<String>,
    headers: HeaderMap,
    Json(payload): Json<Value>,
) -> Result<impl IntoResponse, (StatusCode, Json<Value>)> {
    require_admin(&state, &headers).await?;

    let jobs = get_cron_jobs(&state.db).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "code": 500, "message": e.to_string() })),
        )
    })?;

    let mut existing = jobs
        .into_iter()
        .find(|j| j.id == id)
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(json!({ "code": 404, "message": format!("Cron job '{}' not found", id) })),
            )
        })?;

    if let Some(n) = payload.get("name").and_then(|v| v.as_str()) {
        if !n.trim().is_empty() {
            existing.name = n.trim().to_string();
        }
    }

    if let Some(s) = payload.get("schedule").and_then(|v| v.as_str()) {
        if !s.trim().is_empty() {
            let valid = validate_cron_schedule(s).map_err(|e| {
                (
                    StatusCode::BAD_REQUEST,
                    Json(json!({ "code": 400, "message": format!("Invalid cron schedule: {}", e) })),
                )
            })?;
            existing.schedule = valid;
        }
    }

    if let Some(jt) = payload.get("job_type").and_then(|v| v.as_str()) {
        let jt_lower = jt.trim().to_lowercase();
        if jt_lower == "sql" || jt_lower == "http" {
            existing.job_type = jt_lower;
        }
    }

    if let Some(en) = payload.get("enabled").and_then(|v| v.as_bool()) {
        existing.enabled = en;
    }

    if let Some(sq) = payload.get("sql_query") {
        existing.sql_query = sq.as_str().map(|s| s.trim().to_string());
    }

    if let Some(hm) = payload.get("http_method") {
        existing.http_method = hm.as_str().map(|s| s.trim().to_uppercase());
    }

    if let Some(hu) = payload.get("http_url") {
        existing.http_url = hu.as_str().map(|s| s.trim().to_string());
    }

    if let Some(hh) = payload.get("http_headers") {
        existing.http_headers = serde_json::from_value(hh.clone()).ok();
    }

    if let Some(hb) = payload.get("http_body") {
        existing.http_body = hb.as_str().map(|s| s.to_string());
    }

    existing.updated_at = Utc::now().to_rfc3339();

    upsert_cron_job(&state.db, existing.clone())
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "code": 500, "message": e.to_string() })),
            )
        })?;

    Ok(Json(json!(existing)))
}

/// DELETE /api/cron/jobs/:id - Delete a cron job.
pub async fn delete_job(
    State(state): State<AppState>,
    Path(id): Path<String>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, (StatusCode, Json<Value>)> {
    require_admin(&state, &headers).await?;

    let deleted = delete_cron_job(&state.db, &id).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "code": 500, "message": e.to_string() })),
        )
    })?;

    if deleted {
        Ok(Json(json!({
            "code": 200,
            "message": format!("Cron job '{}' deleted successfully", id)
        })))
    } else {
        Err((
            StatusCode::NOT_FOUND,
            Json(json!({ "code": 404, "message": format!("Cron job '{}' not found", id) })),
        ))
    }
}

/// POST /api/cron/jobs/:id/run - Immediately trigger manual execution of a cron job.
pub async fn run_job_now(
    State(state): State<AppState>,
    Path(id): Path<String>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, (StatusCode, Json<Value>)> {
    require_admin(&state, &headers).await?;

    let res = execute_cron_job(&state.db, &id).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "code": 500, "message": e.to_string() })),
        )
    })?;

    let is_ok = res.status == "success";
    let status_code = if is_ok { StatusCode::OK } else { StatusCode::BAD_REQUEST };

    Ok((
        status_code,
        Json(json!(res)),
    ))
}
