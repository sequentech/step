//! Main server binary for Trellis
//!
//! Runs the HTTP API server and batch processor that monitors `PostgreSQL` tables
//! and builds merkle trees for Certificate Transparency-style verification.
//!

#![allow(clippy::print_stdout)]
//! # Usage
//!
//! ```bash
//! cargo run --bin main
//! cargo run --bin main -- --verify-db
//! cargo run --bin main -- --workers 8
//! ```
//!
//! The `--verify-db` flag validates the database schema without starting the server.

use anyhow::Result;
use std::net::SocketAddr;
use std::str::FromStr;
use trellis::service;

fn main() -> Result<()> {
    dotenv::dotenv().ok();

    // Parse command line arguments
    let args: Vec<String> = std::env::args().collect();
    let verify_db = args.contains(&"--verify-db".to_string());

    let worker_threads = num_cpus::get();
    // Build tokio runtime with configurable parameters
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(worker_threads)
        .enable_all()
        .thread_name("trellis-worker")
        // Increase thread stack size (default is 2MB)
        .thread_stack_size(4 * 1024 * 1024)
        .build()?;

    println!("Tokio runtime: {worker_threads} worker threads");

    runtime.block_on(async_main(verify_db))
}

/// Server entry point
async fn async_main(verify_db: bool) -> Result<()> {
    // Initialize tracing subscriber
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    // Initialize application state (database connection + merkle state)
    let app_state = service::initialize_app_state().await?;

    // If --verify-db flag is present, run validation and exit
    if verify_db {
        let conn = app_state.db_pool.get().await?;
        let validations = service::validate_all_logs(&conn).await?;
        service::print_validation_report(&validations);

        // Exit with status code based on validation result
        let all_valid = validations
            .iter()
            .all(trellis::service::LogValidation::is_valid);
        std::process::exit(i32::from(!all_valid));
    }

    let addr_string =
        std::env::var("TRELLIS_SERVER_ADDR").unwrap_or_else(|_| "127.0.0.1:3000".to_string());
    let addr = SocketAddr::from_str(&addr_string)?;

    // Run the complete server with batch processing
    service::run_server(app_state, &addr).await?;

    Ok(())
}
