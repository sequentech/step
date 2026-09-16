// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::Result;
use axum::{
    routing::{get, post},
    Router,
};
use tower_http::cors::{Any, CorsLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use b4::{db, handlers, s3, state::AppState};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "wbraid_service=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Initialize database
    let db = db::init_db().await?;

    // Initialize S3 client
    let s3_client = s3::init_s3_client().await;

    let state = AppState::new(db, s3_client);

    // Configure CORS
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        // Board management
        .route("/boards", post(handlers::create_board))
        .route("/boards", get(handlers::list_boards))
        .route("/boards/:board", get(handlers::get_board))
        // Message operations (board-specific)
        .route("/boards/:board/messages/list", get(handlers::list_messages))
        .route("/boards/:board/messages", get(handlers::get_messages))
        .route("/boards/:board/messages/:id", get(handlers::get_message))
        // POST - S3 two-step flow
        .route(
            "/boards/:board/messages/initiate",
            post(handlers::initiate_message),
        )
        .route(
            "/boards/:board/messages/:id/confirm",
            post(handlers::confirm_message),
        )
        .layer(cors)
        .with_state(state);

    let bind = std::env::var("WBRAID_B4_BIND").unwrap_or_else(|_| "127.0.0.1:3000".to_string());
    let listener = tokio::net::TcpListener::bind(&bind).await?;
    tracing::info!(
        "Bulletin board service listening on {}",
        listener.local_addr()?
    );

    axum::serve(listener, app).await?;

    Ok(())
}
