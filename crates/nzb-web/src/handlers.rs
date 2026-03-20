use std::sync::Arc;

use axum::extract::{Multipart, Path, Query, State};
use axum::response::IntoResponse;
use axum::Json;
use http::StatusCode;
use serde::{Deserialize, Serialize};

use nzb_core::models::*;
use nzb_core::nzb_parser;

use crate::error::ApiError;
use crate::state::AppState;

// ---------------------------------------------------------------------------
// Query parameters
// ---------------------------------------------------------------------------

#[derive(Deserialize, Default)]
pub struct QueueQuery {
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

#[derive(Deserialize, Default)]
pub struct HistoryQuery {
    pub limit: Option<usize>,
}

#[derive(Deserialize)]
pub struct AddNzbQuery {
    pub category: Option<String>,
    pub priority: Option<i32>,
    pub name: Option<String>,
}

// ---------------------------------------------------------------------------
// Response types
// ---------------------------------------------------------------------------

#[derive(Serialize)]
pub struct QueueResponse {
    pub jobs: Vec<NzbJob>,
    pub total: usize,
    pub speed_bps: u64,
    pub paused: bool,
}

#[derive(Serialize)]
pub struct HistoryResponse {
    pub entries: Vec<HistoryEntry>,
    pub total: usize,
}

#[derive(Serialize)]
pub struct AddNzbResponse {
    pub status: bool,
    pub nzo_ids: Vec<String>,
}

#[derive(Serialize)]
pub struct StatusResponse {
    pub version: &'static str,
    pub paused: bool,
    pub speed_bps: u64,
    pub queue_size: usize,
    pub disk_space_free: u64,
}

#[derive(Serialize)]
pub struct SimpleResponse {
    pub status: bool,
}

// ---------------------------------------------------------------------------
// Queue handlers
// ---------------------------------------------------------------------------

/// GET /api/queue -- List all jobs in the download queue.
pub async fn h_queue_list(
    State(state): State<Arc<AppState>>,
    Query(_q): Query<QueueQuery>,
) -> Result<Json<QueueResponse>, ApiError> {
    let qm = &state.queue_manager;
    let jobs = qm.get_jobs();
    let total = jobs.len();
    let speed_bps = qm.get_speed();
    let paused = qm.is_paused();

    Ok(Json(QueueResponse {
        jobs,
        total,
        speed_bps,
        paused,
    }))
}

/// POST /api/queue/add -- Add an NZB file to the queue.
pub async fn h_queue_add(
    State(state): State<Arc<AppState>>,
    Query(q): Query<AddNzbQuery>,
    mut multipart: Multipart,
) -> Result<impl IntoResponse, ApiError> {
    let mut nzo_ids = Vec::new();

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| ApiError::from(anyhow::anyhow!("Multipart error: {e}")))?
    {
        let file_name = field
            .file_name()
            .map(|s| s.to_string())
            .unwrap_or_else(|| "unknown.nzb".into());

        let data = field
            .bytes()
            .await
            .map_err(|e| ApiError::from(anyhow::anyhow!("Read error: {e}")))?;

        let name = q.name.clone().unwrap_or_else(|| {
            file_name
                .strip_suffix(".nzb")
                .unwrap_or(&file_name)
                .to_string()
        });

        let mut job = nzb_parser::parse_nzb(&name, &data).map_err(ApiError::from)?;

        // Apply category
        if let Some(ref cat) = q.category {
            job.category = cat.clone();
        }

        // Apply priority
        if let Some(prio) = q.priority {
            job.priority = match prio {
                0 => Priority::Low,
                2 => Priority::High,
                3 => Priority::Force,
                _ => Priority::Normal,
            };
        }

        // Set working directories
        let qm = &state.queue_manager;
        job.work_dir = qm.incomplete_dir().join(&job.id);
        job.output_dir = qm.complete_dir().join(&job.category);

        // Create work directory
        std::fs::create_dir_all(&job.work_dir)
            .map_err(|e| ApiError::from(anyhow::anyhow!("Failed to create work dir: {e}")))?;

        let id = job.id.clone();

        tracing::info!(
            name = %job.name,
            id = %job.id,
            files = job.file_count,
            articles = job.article_count,
            "NZB added to queue"
        );

        // Add to the queue manager (persists to DB and starts downloading)
        qm.add_job(job).map_err(ApiError::from)?;
        nzo_ids.push(id);
    }

    Ok((
        StatusCode::OK,
        Json(AddNzbResponse {
            status: true,
            nzo_ids,
        }),
    ))
}

/// POST /api/queue/{id}/pause -- Pause a job.
pub async fn h_queue_pause(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<SimpleResponse>, ApiError> {
    state.queue_manager.pause_job(&id).map_err(ApiError::from)?;
    Ok(Json(SimpleResponse { status: true }))
}

/// POST /api/queue/{id}/resume -- Resume a paused job.
pub async fn h_queue_resume(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<SimpleResponse>, ApiError> {
    state
        .queue_manager
        .resume_job(&id)
        .map_err(ApiError::from)?;
    Ok(Json(SimpleResponse { status: true }))
}

/// DELETE /api/queue/{id} -- Remove a job from the queue.
pub async fn h_queue_delete(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<SimpleResponse>, ApiError> {
    state
        .queue_manager
        .remove_job(&id)
        .map_err(ApiError::from)?;
    Ok(Json(SimpleResponse { status: true }))
}

/// POST /api/queue/pause -- Pause all downloads.
pub async fn h_queue_pause_all(
    State(state): State<Arc<AppState>>,
) -> Result<Json<SimpleResponse>, ApiError> {
    state.queue_manager.pause_all();
    Ok(Json(SimpleResponse { status: true }))
}

/// POST /api/queue/resume -- Resume all downloads.
pub async fn h_queue_resume_all(
    State(state): State<Arc<AppState>>,
) -> Result<Json<SimpleResponse>, ApiError> {
    state.queue_manager.resume_all();
    Ok(Json(SimpleResponse { status: true }))
}

// ---------------------------------------------------------------------------
// History handlers
// ---------------------------------------------------------------------------

/// GET /api/history -- List completed/failed jobs.
pub async fn h_history_list(
    State(state): State<Arc<AppState>>,
    Query(q): Query<HistoryQuery>,
) -> Result<Json<HistoryResponse>, ApiError> {
    let limit = q.limit.unwrap_or(50);
    let entries = state
        .queue_manager
        .history_list(limit)
        .map_err(ApiError::from)?;
    let total = entries.len();
    Ok(Json(HistoryResponse { entries, total }))
}

/// DELETE /api/history/{id} -- Remove a history entry.
pub async fn h_history_delete(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<SimpleResponse>, ApiError> {
    state
        .queue_manager
        .history_remove(&id)
        .map_err(ApiError::from)?;
    Ok(Json(SimpleResponse { status: true }))
}

/// DELETE /api/history -- Clear all history.
pub async fn h_history_clear(
    State(state): State<Arc<AppState>>,
) -> Result<Json<SimpleResponse>, ApiError> {
    state
        .queue_manager
        .history_clear()
        .map_err(ApiError::from)?;
    Ok(Json(SimpleResponse { status: true }))
}

// ---------------------------------------------------------------------------
// Status handler
// ---------------------------------------------------------------------------

/// GET /api/status -- Overall application status.
pub async fn h_status(
    State(state): State<Arc<AppState>>,
) -> Result<Json<StatusResponse>, ApiError> {
    let qm = &state.queue_manager;
    Ok(Json(StatusResponse {
        version: env!("CARGO_PKG_VERSION"),
        paused: qm.is_paused(),
        speed_bps: qm.get_speed(),
        queue_size: qm.queue_size(),
        disk_space_free: get_disk_space_free(&state.config.general.complete_dir),
    }))
}

// ---------------------------------------------------------------------------
// Config handlers
// ---------------------------------------------------------------------------

/// GET /api/config -- Get current configuration.
pub async fn h_config_get(
    State(state): State<Arc<AppState>>,
) -> Result<Json<nzb_core::config::AppConfig>, ApiError> {
    Ok(Json(state.config.clone()))
}

/// GET /api/config/servers -- List configured servers.
pub async fn h_servers_list(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<nzb_core::config::ServerConfig>>, ApiError> {
    Ok(Json(state.config.servers.clone()))
}

/// GET /api/config/categories -- List configured categories.
pub async fn h_categories_list(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<nzb_core::config::CategoryConfig>>, ApiError> {
    Ok(Json(state.config.categories.clone()))
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Get free disk space for a path (returns 0 on error).
fn get_disk_space_free(_path: &std::path::Path) -> u64 {
    // TODO: implement platform-specific disk space check
    0
}
