use std::path::PathBuf;
use std::sync::Arc;

use clap::Parser;
use tracing::info;
use tracing_subscriber::EnvFilter;

use nzb_core::config::AppConfig;
use nzb_core::db::Database;
use nzb_web::{AppState, QueueManager};

#[derive(Parser, Debug)]
#[command(name = "rustnzbd", version, about = "Usenet NZB download client")]
struct Args {
    /// Path to config file
    #[arg(short, long, default_value = "config.toml", env = "RUSTNZBD_CONFIG")]
    config: PathBuf,

    /// Override listen address
    #[arg(long, env = "RUSTNZBD_LISTEN_ADDR")]
    listen_addr: Option<String>,

    /// Override listen port
    #[arg(short, long, env = "RUSTNZBD_PORT")]
    port: Option<u16>,

    /// Override data directory
    #[arg(long, env = "RUSTNZBD_DATA_DIR")]
    data_dir: Option<PathBuf>,

    /// Log level (trace, debug, info, warn, error)
    #[arg(long, default_value = "info", env = "RUSTNZBD_LOG_LEVEL")]
    log_level: String,

    /// Log file path
    #[arg(long, env = "RUSTNZBD_LOG_FILE")]
    log_file: Option<PathBuf>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    // Initialize logging
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(&args.log_level));

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(true)
        .init();

    info!("rustnzbd v{}", env!("CARGO_PKG_VERSION"));

    // Load configuration
    let mut config = AppConfig::load(&args.config)?;

    // Apply CLI overrides
    if let Some(addr) = args.listen_addr {
        config.general.listen_addr = addr;
    }
    if let Some(port) = args.port {
        config.general.port = port;
    }
    if let Some(data_dir) = args.data_dir {
        config.general.data_dir = data_dir;
    }

    // Ensure directories exist
    std::fs::create_dir_all(&config.general.data_dir)?;
    std::fs::create_dir_all(&config.general.incomplete_dir)?;
    std::fs::create_dir_all(&config.general.complete_dir)?;

    // Open database
    let db_path = config.general.data_dir.join("rustnzbd.db");
    let db = Database::open(&db_path)?;
    info!(path = %db_path.display(), "Database opened");

    // Create the queue manager with server configs
    let queue_manager = QueueManager::new(
        config.servers.clone(),
        db,
        config.general.incomplete_dir.clone(),
        config.general.complete_dir.clone(),
    );

    // Restore any in-progress jobs from the database
    if let Err(e) = queue_manager.restore_from_db() {
        tracing::warn!("Failed to restore queue from database: {e}");
    }

    // Spawn the speed tracker background task
    queue_manager.spawn_speed_tracker();

    info!(
        servers = config.servers.len(),
        "Queue manager initialized"
    );

    // Build shared application state
    let state = Arc::new(AppState::new(config, Arc::clone(&queue_manager)));

    // Start HTTP server
    info!("Starting HTTP API server");
    nzb_web::run(state).await?;

    Ok(())
}
