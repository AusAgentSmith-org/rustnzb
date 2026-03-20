use std::path::PathBuf;
use std::sync::Arc;

use arc_swap::ArcSwap;
use nzb_core::config::AppConfig;

use crate::log_buffer::LogBuffer;
use crate::queue_manager::QueueManager;

/// Shared application state, accessible from all HTTP handlers.
pub struct AppState {
    pub config: ArcSwap<AppConfig>,
    pub config_path: PathBuf,
    pub queue_manager: Arc<QueueManager>,
    pub log_buffer: LogBuffer,
}

impl AppState {
    pub fn new(
        config: AppConfig,
        config_path: PathBuf,
        queue_manager: Arc<QueueManager>,
        log_buffer: LogBuffer,
    ) -> Self {
        Self {
            config: ArcSwap::new(Arc::new(config)),
            config_path,
            queue_manager,
            log_buffer,
        }
    }

    /// Get current config snapshot.
    pub fn config(&self) -> Arc<AppConfig> {
        self.config.load_full()
    }

    /// Update config in memory and save to file.
    pub fn update_config(&self, config: AppConfig) -> anyhow::Result<()> {
        config.save(&self.config_path)?;
        self.config.store(Arc::new(config));
        Ok(())
    }
}
