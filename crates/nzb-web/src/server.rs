use std::sync::Arc;

use axum::extract::Path;
use axum::response::{IntoResponse, Response};
use axum::routing::{delete, get, post};
use axum::Router;
use http::{header, StatusCode};
use rust_embed::Embed;
use tokio::net::TcpListener;
use tower_http::cors::{AllowHeaders, AllowOrigin, CorsLayer};
use tower_http::trace::TraceLayer;
use tracing::info;

use crate::handlers;
use crate::sabnzbd_compat;
use crate::state::AppState;

/// Embed the static/ directory at compile time.
#[derive(Embed)]
#[folder = "static/"]
struct StaticAssets;

/// Serve the root page (index.html) from embedded static assets.
async fn h_root() -> Response {
    serve_embedded_file("index.html")
}

/// Serve any file from the embedded static assets by path.
async fn h_static(Path(path): Path<String>) -> Response {
    serve_embedded_file(&path)
}

/// Look up an embedded file and return it with the correct Content-Type.
fn serve_embedded_file(path: &str) -> Response {
    match StaticAssets::get(path) {
        Some(content) => {
            let mime = mime_guess::from_path(path).first_or_octet_stream();
            (
                StatusCode::OK,
                [(header::CONTENT_TYPE, mime.as_ref().to_string())],
                content.data.into_owned(),
            )
                .into_response()
        }
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

/// Build the axum Router with all API routes.
pub fn build_router(state: Arc<AppState>) -> Router {
    let cors = CorsLayer::default()
        .allow_origin(AllowOrigin::any())
        .allow_headers(AllowHeaders::any());

    // Native REST API
    let api_routes = Router::new()
        // Status
        .route("/status", get(handlers::h_status))
        // Queue
        .route("/queue", get(handlers::h_queue_list))
        .route("/queue/add", post(handlers::h_queue_add))
        .route("/queue/pause", post(handlers::h_queue_pause_all))
        .route("/queue/resume", post(handlers::h_queue_resume_all))
        .route("/queue/{id}/pause", post(handlers::h_queue_pause))
        .route("/queue/{id}/resume", post(handlers::h_queue_resume))
        .route("/queue/{id}", delete(handlers::h_queue_delete))
        // History
        .route("/history", get(handlers::h_history_list))
        .route("/history/{id}", delete(handlers::h_history_delete))
        .route("/history", delete(handlers::h_history_clear))
        // Config
        .route("/config", get(handlers::h_config_get))
        .route("/config/servers", get(handlers::h_servers_list))
        .route("/config/categories", get(handlers::h_categories_list));

    // Arr-compatible API (Sonarr/Radarr)
    let sabnzbd_route = Router::new()
        .route("/sabnzbd/api", get(sabnzbd_compat::h_sabnzbd_api_get))
        .route("/sabnzbd/api", post(sabnzbd_compat::h_sabnzbd_api_post));

    Router::new()
        .route("/", get(h_root))
        .route("/static/{*path}", get(h_static))
        .nest("/api", api_routes)
        .merge(sabnzbd_route)
        .layer(TraceLayer::new_for_http())
        .layer(cors)
        .with_state(state)
}

/// Start the HTTP server.
pub async fn run(state: Arc<AppState>) -> anyhow::Result<()> {
    let addr = format!(
        "{}:{}",
        state.config.general.listen_addr, state.config.general.port
    );

    let router = build_router(state);
    let listener = TcpListener::bind(&addr).await?;

    info!("HTTP server listening on http://{addr}");
    info!("Web GUI: http://{addr}/");
    info!("Arr API: http://{addr}/sabnzbd/api?mode=version");

    axum::serve(listener, router).await?;
    Ok(())
}
