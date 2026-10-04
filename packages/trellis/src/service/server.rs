use anyhow::Result;
use axum::{Json, Router, http::StatusCode, routing::get};
use dashmap::DashMap;
use deadpool_postgres::{Config, ManagerConfig, RecyclingMethod, Runtime};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio_postgres::NoTls;

use crate::service::state::AppState;

/// Creates the public, read-only proof server. Administrative controls are excluded.
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
        .with_state(app_state)
        .fallback(handle_unmatched)
}

/// Creates administrative controls for a separately secured listener.
///
/// `run_server` binds this router only to an explicitly configured loopback address.
/// Embedders must enforce their own access boundary when using this router.
pub fn create_admin_server(app_state: AppState) -> Router {
    Router::new()
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
}

/// Rejects accidental exposure of unauthenticated controls beyond the host.
fn validate_admin_address(addr: SocketAddr) -> Result<SocketAddr> {
    anyhow::ensure!(
        addr.ip().is_loopback(),
        "TRELLIS_ADMIN_ADDR must be a loopback address"
    );
    Ok(addr)
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
    // Bind all requested listeners before starting background processing.
    let listener = tokio::net::TcpListener::bind(addr).await?;
    let admin_listener = match std::env::var("TRELLIS_ADMIN_ADDR") {
        Ok(value) => {
            Some(tokio::net::TcpListener::bind(validate_admin_address(value.parse()?)?).await?)
        }
        Err(std::env::VarError::NotPresent) => None,
        Err(error) => return Err(error.into()),
    };
    let admin_app = create_admin_server(app_state.clone());
    let admin_server = async move {
        if let Some(listener) = admin_listener {
            tracing::info!(address = %listener.local_addr()?, "Administrative server listening on loopback");
            axum::serve(listener, admin_app).await
        } else {
            std::future::pending::<std::io::Result<()>>().await
        }
    };
    // Clone the state for the processing task
    let process_state = app_state.clone();

    // Rebuild all logs from database on startup to ensure in-memory trees match persistent state
    crate::service::rebuild_all_logs(&app_state).await?;

    // Spawn the batch processing task
    let mut processor = tokio::spawn(async move {
        crate::service::processor::run_batch_processor(process_state).await;
    });

    // Create the server
    let app = create_server(app_state);

    tracing::info!(address = %addr, "HTTP server listening");
    let server = axum::serve(listener, app);

    // Wait for Ctrl+C, processor, or server to finish
    tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            tracing::info!("Received Ctrl+C, shutting down");
        }
        result = &mut processor => {
            match result {
                Ok(()) => tracing::info!("Processor completed successfully"),
                Err(e) => tracing::error!(error = ?e, "Processor error"),
            }
        }
        result = server => {
            processor.abort();
            result?;
        }
        result = admin_server => {
            processor.abort();
            result?;
        }
    }

    processor.abort();
    Ok(())
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use reqwest::StatusCode;

    pub(crate) fn test_state(url: &str) -> AppState {
        let mut config = Config::new();
        config.url = Some(url.to_owned());
        // Two source writers plus the processor must fit even on a one-CPU host.
        config.pool = Some(deadpool_postgres::PoolConfig::new(4));
        AppState {
            merkle_states: Arc::new(DashMap::new()),
            db_pool: config
                .create_pool(Some(Runtime::Tokio1), NoTls)
                .expect("pool"),
            metrics: Arc::new(crate::service::metrics::Metrics::new()),
            http_metrics: Arc::new(crate::service::metrics::HttpMetrics::new()),
            processor_state: Arc::new(std::sync::atomic::AtomicU8::new(0)),
        }
    }

    #[test]
    fn administrative_listener_requires_loopback() {
        for address in ["0.0.0.0:3001", "[::]:3001", "192.0.2.1:3001"] {
            assert!(validate_admin_address(address.parse().unwrap()).is_err());
        }
        for address in ["127.0.0.1:3001", "[::1]:3001"] {
            assert!(validate_admin_address(address.parse().unwrap()).is_ok());
        }
    }

    #[tokio::test]
    async fn public_routes_cannot_control_the_processor() {
        let state = test_state("postgres://unused@127.0.0.1/unused");
        let mut merkle_state = crate::service::MerkleState::new();
        merkle_state.update_with_entry(crate::LeafHash::new(vec![1; 32]), 1);
        state.merkle_states.insert(
            "test".into(),
            Arc::new(parking_lot::RwLock::new(merkle_state)),
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let app = create_server(state.clone());
        let task = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        let client = reqwest::Client::new();
        for route in ["pause", "resume", "stop"] {
            assert_eq!(
                client
                    .post(format!("http://{address}/admin/{route}"))
                    .send()
                    .await
                    .unwrap()
                    .status(),
                StatusCode::NOT_FOUND
            );
            assert_eq!(
                state
                    .processor_state
                    .load(std::sync::atomic::Ordering::Relaxed),
                0
            );
        }
        assert_eq!(
            client
                .get(format!("http://{address}/metrics"))
                .send()
                .await
                .unwrap()
                .status(),
            StatusCode::OK
        );
        assert_eq!(
            client
                .get(format!("http://{address}/admin/status"))
                .send()
                .await
                .unwrap()
                .status(),
            StatusCode::NOT_FOUND
        );
        assert_eq!(
            client
                .get(format!("http://{address}/logs/test/root"))
                .send()
                .await
                .unwrap()
                .status(),
            StatusCode::OK
        );
        task.abort();

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let app = create_admin_server(state.clone());
        let task = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        for (route, expected) in [("pause", 1), ("resume", 0), ("stop", 2)] {
            assert_eq!(
                client
                    .post(format!("http://{address}/admin/{route}"))
                    .send()
                    .await
                    .unwrap()
                    .status(),
                StatusCode::OK
            );
            assert_eq!(
                state
                    .processor_state
                    .load(std::sync::atomic::Ordering::Relaxed),
                expected
            );
        }
        task.abort();
    }
}
