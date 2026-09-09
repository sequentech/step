// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::{Context, Result};
use axum::{
    routing::{get, post},
    Router,
};
use dotenv::dotenv;
use sequent_core::util::init_log::init_log;
use std::env;
use tower_http::cors::{Any, CorsLayer};

use b4::{db, handlers, s3, state::AppState};

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    init_log(true);

    let b4_bind = env::var("B4_BIND").context("B4_BIND must be set")?;

    // Initialize PostgreSQL database
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
        .layer(cors)
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(&b4_bind).await?;
    tracing::info!(
        "Bulletin board service listening on {}",
        listener.local_addr()?
    );

    axum::serve(listener, app).await?;

    Ok(())
}
