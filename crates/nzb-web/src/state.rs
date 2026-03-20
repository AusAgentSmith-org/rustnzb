use std::sync::Arc;

use nzb_core::config::AppConfig;

use crate::queue_manager::QueueManager;

/// Shared application state, accessible from all HTTP handlers.
pub struct AppState {
    pub config: AppConfig,
    pub queue_manager: Arc<QueueManager>,
}

impl AppState {
    pub fn new(config: AppConfig, queue_manager: Arc<QueueManager>) -> Self {
        Self {
            config,
            queue_manager,
        }
    }
}
