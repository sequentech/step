// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::{Context, Result};
use axum::{
    http::Method,
    routing::{get, post},
    Router,
};
use dotenv::dotenv;
use sequent_core::types::env_vars as ev;
use sequent_core::util::init_log::init_log;
use std::env;
use tower_http::cors::{Any, CorsLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use b4::{db, handlers, s3, state::AppState};

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    init_log(true);

    let b4_bind = env::var(ev::B4_BIND).context("B4_BIND must be set")?;
    // Initialize PostgreSQL database
    let db = db::init_db().await?;
    // Initialize S3 client
    let s3_client = s3::init_s3_client().await;
    let state = AppState::new(db, s3_client);
    let is_dev_env = env::var(ev::ENV_SLUG).unwrap_or_else(|_| "".to_string()) == "dev";

    // Configure CORS
    // - development: default allow all origins to unblock local browser work
    // - production: require an explicit allowlist via B4_ALLOWED_ORIGINS
    let allowed_origins_str = env::var(ev::B4_ALLOWED_ORIGINS).unwrap_or_else(|_| "*".to_string());

    let cors = if allowed_origins_str.trim() == "*" {
        if !is_dev_env {
            anyhow::bail!("B4_ALLOWED_ORIGINS cannot be '*' in production")
        }

        tracing::warn!("CORS: Allowing all origins (*) - development mode");
        CorsLayer::new()
            .allow_origin(Any)
            .allow_methods(Any)
            .allow_headers(Any)
    } else {
        let origins: Vec<_> = allowed_origins_str
            .split(',')
            .filter_map(|s| {
                let trimmed = s.trim();
                if trimmed.is_empty() {
                    None
                } else {
                    trimmed.parse().ok()
                }
            })
            .collect();

        if origins.is_empty() {
            anyhow::bail!("B4_ALLOWED_ORIGINS did not contain any valid origins")
        }

        tracing::info!("CORS: Allowing origins: {:?}", origins);

        CorsLayer::new()
            .allow_origin(origins)
            .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
            .allow_headers(Any)
    };

    let app = Router::new()
        // Board management
        .route("/boards", post(handlers::create_board))
        .route("/boards", get(handlers::list_boards))
        .route("/boards/:board", get(handlers::get_board))
        // Message operations (board-specific)
        .route(
            "/boards/:board/messages/initiate",
            post(handlers::initiate_message),
        )
        .route(
            "/boards/:board/messages/:id/confirm",
            post(handlers::confirm_message),
        )
        .route("/boards/:board/messages/list", get(handlers::list_messages))
        .route("/boards/:board/messages", get(handlers::get_messages))
        .route("/boards/:board/messages/:id", get(handlers::get_message))
        // Multi-board operations (GET)
        .route(
            "/boards/messages/multi/get",
            post(handlers::get_messages_multi),
        )
        // Multi-board operations (PUT - S3 two-step flow)
        .route(
            "/boards/messages/multi/initiate",
            post(handlers::initiate_messages_multi),
        )
        .route(
            "/boards/messages/multi/confirm",
            post(handlers::confirm_messages_multi),
        )
        // Trustee session heartbeat
        .route("/boards/:board/sessions/heartbeat", post(handlers::post_heartbeat))
        .route("/sessions", get(handlers::get_sessions))
        .layer(cors)
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(&b4_bind).await?;
    tracing::info!(
        "Bulletin board service listening on {}",
        listener.local_addr()?
    );
    tracing::info!("JWT authentication is ENABLED - all endpoints require valid trustee role");

    axum::serve(listener, app).await?;

    Ok(())
}
