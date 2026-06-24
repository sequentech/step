// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use axum::{
    routing::{get, post},
    Router,
};

use crate::{handlers, state::AppState};

/// Builds the B4 HTTP route table. State and middleware (CORS) are applied by
/// the caller so each environment can configure them independently.
pub fn build_router() -> Router<AppState> {
    Router::new()
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
}
