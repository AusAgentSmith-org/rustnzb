//! Queue manager — coordinates downloads across the application.
//!
//! The QueueManager owns the list of active NzbJobs, manages the download
//! engine instances, and exposes a thread-safe API for the HTTP handlers
//! to interact with.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use parking_lot::Mutex;
use tokio::sync::mpsc;
use tracing::{error, info, warn};

use nzb_core::config::ServerConfig;
use nzb_core::db::Database;
use nzb_core::models::*;

use crate::download_engine::{DownloadEngine, ProgressUpdate};

// ---------------------------------------------------------------------------
// Speed tracker (simple rolling window)
// ---------------------------------------------------------------------------

struct SpeedTracker {
    /// Bytes downloaded in the current window.
    window_bytes: AtomicU64,
    /// Current speed in bytes per second.
    current_bps: AtomicU64,
}

impl SpeedTracker {
    fn new() -> Self {
        Self {
            window_bytes: AtomicU64::new(0),
            current_bps: AtomicU64::new(0),
        }
    }

    /// Record downloaded bytes.
    fn record(&self, bytes: u64) {
        self.window_bytes.fetch_add(bytes, Ordering::Relaxed);
    }

    /// Called periodically to compute speed and reset the window.
    fn tick(&self, elapsed_secs: f64) {
        let bytes = self.window_bytes.swap(0, Ordering::Relaxed);
        if elapsed_secs > 0.001 {
            let bps = (bytes as f64 / elapsed_secs) as u64;
            self.current_bps.store(bps, Ordering::Relaxed);
        }
    }

    fn bps(&self) -> u64 {
        self.current_bps.load(Ordering::Relaxed)
    }
}

// ---------------------------------------------------------------------------
// Per-job state
// ---------------------------------------------------------------------------

struct JobState {
    /// The job data (shared with API for reading).
    job: NzbJob,
    /// The download engine for this job.
    engine: Arc<DownloadEngine>,
    /// Handle to the download task (so we can await or abort it).
    task_handle: Option<tokio::task::JoinHandle<()>>,
}

// ---------------------------------------------------------------------------
// QueueManager
// ---------------------------------------------------------------------------

/// Thread-safe queue manager that coordinates all downloads.
///
/// Wrapped in `Arc` for sharing between the background task and HTTP handlers.
pub struct QueueManager {
    /// Active jobs keyed by job ID.
    jobs: Mutex<HashMap<String, JobState>>,
    /// Order of job IDs for display.
    job_order: Mutex<Vec<String>>,
    /// Server configurations.
    servers: Mutex<Vec<ServerConfig>>,
    /// Whether all downloads are globally paused.
    globally_paused: AtomicBool,
    /// Speed tracker.
    speed: SpeedTracker,
    /// Database for persistence.
    db: Mutex<Database>,
    /// App config (incomplete_dir, complete_dir).
    incomplete_dir: std::path::PathBuf,
    complete_dir: std::path::PathBuf,
}

impl QueueManager {
    /// Create a new queue manager.
    pub fn new(
        servers: Vec<ServerConfig>,
        db: Database,
        incomplete_dir: std::path::PathBuf,
        complete_dir: std::path::PathBuf,
    ) -> Arc<Self> {
        Arc::new(Self {
            jobs: Mutex::new(HashMap::new()),
            job_order: Mutex::new(Vec::new()),
            servers: Mutex::new(servers),
            globally_paused: AtomicBool::new(false),
            speed: SpeedTracker::new(),
            db: Mutex::new(db),
            incomplete_dir,
            complete_dir,
        })
    }

    /// Add a job to the queue and start downloading.
    ///
    /// The job should already have its `work_dir` and `output_dir` set.
    pub fn add_job(self: &Arc<Self>, mut job: NzbJob) -> nzb_core::Result<()> {
        // Ensure work directory exists
        std::fs::create_dir_all(&job.work_dir)?;

        // Persist to DB
        {
            let db = self.db.lock();
            db.queue_insert(&job)?;
        }

        let job_id = job.id.clone();
        info!(
            job_id = %job_id,
            name = %job.name,
            files = job.file_count,
            articles = job.article_count,
            "Job added to queue"
        );

        // If globally paused, add as paused
        if self.globally_paused.load(Ordering::Relaxed) {
            job.status = JobStatus::Paused;
            let engine = Arc::new(DownloadEngine::new());
            engine.pause();
            let state = JobState {
                job,
                engine,
                task_handle: None,
            };
            self.jobs.lock().insert(job_id.clone(), state);
            self.job_order.lock().push(job_id);
            return Ok(());
        }

        // Start downloading
        job.status = JobStatus::Downloading;
        self.start_download(job);
        Ok(())
    }

    /// Start the download for a job.
    fn start_download(self: &Arc<Self>, job: NzbJob) {
        let job_id = job.id.clone();
        let engine = Arc::new(DownloadEngine::new());
        let (progress_tx, progress_rx) = mpsc::unbounded_channel();

        let servers = self.servers.lock().clone();
        let engine_clone = Arc::clone(&engine);
        let job_clone = job.clone();

        // Spawn the download task
        let task_handle = tokio::spawn(async move {
            engine_clone.run(&job_clone, &servers, progress_tx).await;
        });

        let state = JobState {
            job,
            engine,
            task_handle: Some(task_handle),
        };

        self.jobs.lock().insert(job_id.clone(), state);
        {
            let mut order = self.job_order.lock();
            if !order.contains(&job_id) {
                order.push(job_id.clone());
            }
        }

        // Spawn the progress handler
        let qm = Arc::clone(self);
        tokio::spawn(async move {
            qm.handle_progress(job_id, progress_rx).await;
        });
    }

    /// Handle progress updates from the download engine.
    async fn handle_progress(
        self: Arc<Self>,
        job_id: String,
        mut progress_rx: mpsc::UnboundedReceiver<ProgressUpdate>,
    ) {
        let mut last_db_update = Instant::now();

        while let Some(update) = progress_rx.recv().await {
            match update {
                ProgressUpdate::ArticleComplete {
                    file_id,
                    segment_number,
                    decoded_bytes,
                    file_complete,
                    ..
                } => {
                    self.speed.record(decoded_bytes);

                    // Update in-memory job state
                    {
                        let mut jobs = self.jobs.lock();
                        if let Some(state) = jobs.get_mut(&job_id) {
                            state.job.downloaded_bytes += decoded_bytes;
                            state.job.articles_downloaded += 1;

                            for file in &mut state.job.files {
                                if file.id == file_id {
                                    file.bytes_downloaded += decoded_bytes;
                                    for article in &mut file.articles {
                                        if article.segment_number == segment_number {
                                            article.downloaded = true;
                                            article.data_size = Some(decoded_bytes);
                                        }
                                    }
                                    if file_complete {
                                        file.assembled = true;
                                        state.job.files_completed += 1;
                                        info!(
                                            job_id = %job_id,
                                            file = %file.filename,
                                            completed = state.job.files_completed,
                                            total = state.job.file_count,
                                            "File assembly complete"
                                        );
                                    }
                                    break;
                                }
                            }
                        }
                    }

                    // Batch DB writes (every 2 seconds)
                    if last_db_update.elapsed() >= Duration::from_secs(2) {
                        self.persist_job_progress(&job_id);
                        last_db_update = Instant::now();
                    }
                }
                ProgressUpdate::ArticleFailed { error, .. } => {
                    let mut jobs = self.jobs.lock();
                    if let Some(state) = jobs.get_mut(&job_id) {
                        state.job.articles_failed += 1;
                    }
                    warn!(job_id = %job_id, "Article failed: {error}");
                }
                ProgressUpdate::JobFinished {
                    success,
                    articles_failed,
                    ..
                } => {
                    info!(
                        job_id = %job_id,
                        success,
                        articles_failed,
                        "Job download finished"
                    );
                    self.on_job_finished(&job_id, success, articles_failed);
                    break;
                }
            }
        }
    }

    /// Called when a job's download phase completes.
    fn on_job_finished(&self, job_id: &str, success: bool, articles_failed: usize) {
        // Complete the job and move to history
        {
            let mut jobs = self.jobs.lock();
            if let Some(state) = jobs.get_mut(job_id) {
                if success {
                    state.job.status = JobStatus::PostProcessing;
                    state.job.completed_at = Some(chrono::Utc::now());
                    info!(job_id = %job_id, "Job moving to post-processing");
                } else {
                    state.job.status = JobStatus::Failed;
                    state.job.completed_at = Some(chrono::Utc::now());
                    state.job.error_message = Some(format!(
                        "{articles_failed} article(s) failed to download"
                    ));
                }

                self.move_to_history(state);
            }
        }

        // Persist final state then remove from active queue
        self.persist_job_progress(job_id);

        // Remove from in-memory queue
        self.jobs.lock().remove(job_id);
        self.job_order.lock().retain(|jid| jid != job_id);
    }

    /// Move a job's files to output and insert a history entry.
    fn move_to_history(&self, state: &mut JobState) {
        let final_status = if state.job.articles_failed == 0 {
            JobStatus::Completed
        } else {
            JobStatus::Failed
        };

        // Try to move files from work_dir to output_dir
        if final_status == JobStatus::Completed {
            if let Err(e) = std::fs::create_dir_all(&state.job.output_dir) {
                warn!(job_id = %state.job.id, "Failed to create output dir: {e}");
            }
            if let Ok(entries) = std::fs::read_dir(&state.job.work_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_file() {
                        let dest = state.job.output_dir.join(entry.file_name());
                        if let Err(e) = std::fs::rename(&path, &dest) {
                            if let Err(e2) = std::fs::copy(&path, &dest) {
                                warn!(
                                    job_id = %state.job.id,
                                    file = %path.display(),
                                    "Failed to move file: rename={e}, copy={e2}"
                                );
                            } else {
                                let _ = std::fs::remove_file(&path);
                            }
                        }
                    }
                }
            }
        }

        state.job.status = final_status;

        // Insert into history
        let history_entry = HistoryEntry {
            id: state.job.id.clone(),
            name: state.job.name.clone(),
            category: state.job.category.clone(),
            status: final_status,
            total_bytes: state.job.total_bytes,
            downloaded_bytes: state.job.downloaded_bytes,
            added_at: state.job.added_at,
            completed_at: state.job.completed_at.unwrap_or_else(chrono::Utc::now),
            output_dir: state.job.output_dir.clone(),
            stages: Vec::new(),
            error_message: state.job.error_message.clone(),
        };

        let db = self.db.lock();
        if let Err(e) = db.history_insert(&history_entry) {
            error!(job_id = %state.job.id, "Failed to insert history: {e}");
        }
        if let Err(e) = db.queue_remove(&state.job.id) {
            error!(job_id = %state.job.id, "Failed to remove from queue: {e}");
        }
    }

    /// Persist current job progress to the database.
    fn persist_job_progress(&self, job_id: &str) {
        let jobs = self.jobs.lock();
        if let Some(state) = jobs.get(job_id) {
            let db = self.db.lock();
            if let Err(e) = db.queue_update_progress(
                job_id,
                state.job.status,
                state.job.downloaded_bytes,
                state.job.articles_downloaded,
                state.job.articles_failed,
                state.job.files_completed,
            ) {
                warn!(job_id = %job_id, "Failed to persist progress: {e}");
            }
        }
    }

    // -----------------------------------------------------------------------
    // Job control
    // -----------------------------------------------------------------------

    /// Pause a specific job.
    pub fn pause_job(&self, id: &str) -> nzb_core::Result<()> {
        let mut jobs = self.jobs.lock();
        let state = jobs
            .get_mut(id)
            .ok_or_else(|| nzb_core::NzbError::JobNotFound(id.to_string()))?;

        state.job.status = JobStatus::Paused;
        state.engine.pause();

        let db = self.db.lock();
        db.queue_update_progress(
            id,
            JobStatus::Paused,
            state.job.downloaded_bytes,
            state.job.articles_downloaded,
            state.job.articles_failed,
            state.job.files_completed,
        )?;

        info!(job_id = %id, "Job paused");
        Ok(())
    }

    /// Resume a specific job.
    pub fn resume_job(self: &Arc<Self>, id: &str) -> nzb_core::Result<()> {
        let needs_start = {
            let mut jobs = self.jobs.lock();
            let state = jobs
                .get_mut(id)
                .ok_or_else(|| nzb_core::NzbError::JobNotFound(id.to_string()))?;

            state.job.status = JobStatus::Downloading;
            state.engine.resume();

            if state.task_handle.is_none() {
                // Need to start the download task
                Some(state.job.clone())
            } else {
                let db = self.db.lock();
                let _ = db.queue_update_progress(
                    id,
                    JobStatus::Downloading,
                    state.job.downloaded_bytes,
                    state.job.articles_downloaded,
                    state.job.articles_failed,
                    state.job.files_completed,
                );
                None
            }
        };

        if let Some(job) = needs_start {
            // Remove old state and start fresh
            self.jobs.lock().remove(&job.id);
            self.start_download(job);
        }

        info!(job_id = %id, "Job resumed");
        Ok(())
    }

    /// Remove a specific job from the queue.
    pub fn remove_job(&self, id: &str) -> nzb_core::Result<()> {
        let removed = self.jobs.lock().remove(id);
        if let Some(state) = removed {
            // Cancel the download
            state.engine.cancel();
            if let Some(handle) = state.task_handle {
                handle.abort();
            }

            // Remove from DB
            let db = self.db.lock();
            let _ = db.queue_remove(id);

            // Remove from order
            self.job_order.lock().retain(|jid| jid != id);

            // Try to clean up work directory
            if state.job.work_dir.exists() {
                let _ = std::fs::remove_dir_all(&state.job.work_dir);
            }

            info!(job_id = %id, "Job removed");
        }
        Ok(())
    }

    /// Pause all downloads globally.
    pub fn pause_all(&self) {
        self.globally_paused.store(true, Ordering::Relaxed);
        let jobs = self.jobs.lock();
        for (_id, state) in jobs.iter() {
            if state.job.status == JobStatus::Downloading {
                state.engine.pause();
            }
        }
        info!("All downloads paused");
    }

    /// Resume all downloads globally.
    pub fn resume_all(self: &Arc<Self>) {
        self.globally_paused.store(false, Ordering::Relaxed);

        let jobs_to_start: Vec<NzbJob> = {
            let mut jobs = self.jobs.lock();
            let mut to_start = Vec::new();
            for (_id, state) in jobs.iter_mut() {
                if state.job.status == JobStatus::Paused {
                    state.engine.resume();
                    state.job.status = JobStatus::Downloading;
                    if state.task_handle.is_none() {
                        to_start.push(state.job.clone());
                    }
                }
            }
            to_start
        };

        for job in jobs_to_start {
            self.jobs.lock().remove(&job.id);
            self.start_download(job);
        }

        info!("All downloads resumed");
    }

    // -----------------------------------------------------------------------
    // Query methods (for API handlers)
    // -----------------------------------------------------------------------

    /// Get a snapshot of all jobs in the queue.
    pub fn get_jobs(&self) -> Vec<NzbJob> {
        let jobs = self.jobs.lock();
        let order = self.job_order.lock();
        let mut result = Vec::with_capacity(order.len());
        for id in order.iter() {
            if let Some(state) = jobs.get(id) {
                result.push(state.job.clone());
            }
        }
        result
    }

    /// Get the current download speed in bytes per second.
    pub fn get_speed(&self) -> u64 {
        self.speed.bps()
    }

    /// Check if downloads are globally paused.
    pub fn is_paused(&self) -> bool {
        self.globally_paused.load(Ordering::Relaxed)
    }

    /// Get the number of jobs in the queue.
    pub fn queue_size(&self) -> usize {
        self.jobs.lock().len()
    }

    /// Get a reference to the incomplete_dir.
    pub fn incomplete_dir(&self) -> &std::path::Path {
        &self.incomplete_dir
    }

    /// Get a reference to the complete_dir.
    pub fn complete_dir(&self) -> &std::path::Path {
        &self.complete_dir
    }

    // -----------------------------------------------------------------------
    // History query methods (delegate to DB)
    // -----------------------------------------------------------------------

    /// List history entries.
    pub fn history_list(&self, limit: usize) -> nzb_core::Result<Vec<HistoryEntry>> {
        let db = self.db.lock();
        db.history_list(limit).map_err(Into::into)
    }

    /// Remove a history entry.
    pub fn history_remove(&self, id: &str) -> nzb_core::Result<()> {
        let db = self.db.lock();
        db.history_remove(id).map_err(Into::into)
    }

    /// Clear all history.
    pub fn history_clear(&self) -> nzb_core::Result<()> {
        let db = self.db.lock();
        db.history_clear().map_err(Into::into)
    }

    // -----------------------------------------------------------------------
    // Startup: restore jobs from DB
    // -----------------------------------------------------------------------

    /// Restore in-progress jobs from the database on startup.
    pub fn restore_from_db(self: &Arc<Self>) -> nzb_core::Result<()> {
        let jobs = {
            let db = self.db.lock();
            db.queue_list()?
        };

        if jobs.is_empty() {
            return Ok(());
        }

        info!(count = jobs.len(), "Restoring jobs from database");

        for job in jobs {
            let job_id = job.id.clone();
            let engine = Arc::new(DownloadEngine::new());

            if job.status == JobStatus::Paused {
                engine.pause();
            }

            let state = JobState {
                job,
                engine,
                task_handle: None,
            };
            self.jobs.lock().insert(job_id.clone(), state);
            self.job_order.lock().push(job_id);
        }

        Ok(())
    }

    // -----------------------------------------------------------------------
    // Background task: speed calculation
    // -----------------------------------------------------------------------

    /// Spawn the background task that periodically updates the speed counter.
    pub fn spawn_speed_tracker(self: &Arc<Self>) {
        let qm = Arc::clone(self);
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(1));
            loop {
                interval.tick().await;
                qm.speed.tick(1.0);
            }
        });
    }
}
