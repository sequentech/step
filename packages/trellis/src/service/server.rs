use anyhow::Result;
use axum::{Json, Router, http::StatusCode, routing::get};
use dashmap::DashMap;
use deadpool_postgres::{Config, ManagerConfig, RecyclingMethod, Runtime};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio_postgres::NoTls;

use crate::service::state::AppState;

/// Creates and configures the HTTP server with all routes
pub fn create_server(app_state: AppState) -> Router {
    // Fallback handler for unmatched routes
    async fn handle_unmatched() -> (StatusCode, Json<serde_json::Value>) {
        tracing::debug!("Unmatched route accessed");
        (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "status": "error",
                "error": "Route not found. Available endpoints: /logs/{log_name}/root, /logs/{log_name}/size, /logs/{log_name}/proof, /logs/{log_name}/consistency, /logs/{log_name}/has_leaf, /logs/{log_name}/has_root, /logs/{log_name}/exists"
            })),
        )
    }

    // Build our application with routes and error handling
    Router::new()
        .route("/logs/:log_name/root", get(crate::service::get_merkle_root))
        .route("/logs/:log_name/size", get(crate::service::get_log_size))
        .route(
            "/logs/:log_name/proof",
            get(crate::service::get_inclusion_proof),
        )
        .route(
            "/logs/:log_name/consistency",
            get(crate::service::get_consistency_proof),
        )
        .route("/logs/:log_name/has_leaf", get(crate::service::has_leaf))
        .route("/logs/:log_name/has_root", get(crate::service::has_root))
        .route(
            "/logs/:log_name/exists",
            get(crate::service::routes::has_log),
        )
        .route("/metrics", get(crate::service::routes::metrics))
        .route(
            "/admin/pause",
            axum::routing::post(crate::service::routes::admin_pause),
        )
        .route(
            "/admin/resume",
            axum::routing::post(crate::service::routes::admin_resume),
        )
        .route(
            "/admin/stop",
            axum::routing::post(crate::service::routes::admin_stop),
        )
        .route("/admin/status", get(crate::service::routes::admin_status))
        .with_state(app_state)
        .fallback(handle_unmatched)
}

/// Initialize the application state with database connection pool and empty merkle states map
///
/// # Errors
///
/// Returns an error if the `DATABASE_URL` environment variable is not set or if the
/// database connection pool cannot be created.
pub async fn initialize_app_state() -> Result<AppState> {
    tracing::info!("Connecting to database");

    let db_url = std::env::var("DATABASE_URL")?;

    // Parse connection string into deadpool config
    let mut cfg = Config::new();
    cfg.url = Some(db_url);
    cfg.manager = Some(ManagerConfig {
        recycling_method: RecyclingMethod::Fast,
    });

    // Create connection pool
    let pool = cfg.create_pool(Some(Runtime::Tokio1), NoTls)?;

    // Initialize shared state with empty DashMap for merkle states
    // Logs will be loaded dynamically from verification_logs table
    let app_state = AppState {
        merkle_states: Arc::new(DashMap::new()),
        db_pool: pool,
        metrics: Arc::new(crate::service::metrics::Metrics::new()),
        http_metrics: Arc::new(crate::service::metrics::HttpMetrics::new()),
        processor_state: Arc::new(std::sync::atomic::AtomicU8::new(0)), // 0 = Running
    };

    Ok(app_state)
}

/// Run the complete HTTP server with batch processing
///
/// # Errors
///
/// Returns an error if rebuilding logs from the database fails or if the TCP listener
/// cannot bind to the specified address.
pub async fn run_server(app_state: AppState, addr: &SocketAddr) -> Result<()> {
    // Clone the state for the processing task
    let process_state = app_state.clone();

    // Rebuild all logs from database on startup to ensure in-memory trees match persistent state
    crate::service::rebuild_all_logs(&app_state).await?;

    // Spawn the batch processing task
    let processor = tokio::spawn(async move {
        crate::service::processor::run_batch_processor(process_state).await;
    });

    // Create the server
    let app = create_server(app_state);

    tracing::info!(address = %addr, "HTTP server listening");
    let server = axum::serve(tokio::net::TcpListener::bind(addr).await?, app);

    // Wait for Ctrl+C, processor, or server to finish
    tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            tracing::info!("Received Ctrl+C, shutting down");
        }
        result = processor => {
            match result {
                Ok(()) => tracing::info!("Processor completed successfully"),
                Err(e) => tracing::error!(error = ?e, "Processor error"),
            }
        }
        _ = server => {
            tracing::info!("HTTP server shut down");
        }
    }

    Ok(())
}
