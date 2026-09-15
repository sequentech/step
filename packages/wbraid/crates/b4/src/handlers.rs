// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! HTTP handlers for the b4 bulletin board (§8).
//!
//! b4 is a dumb, board-agnostic blob store: it stores and serves opaque messages
//! and NEVER interprets their contents. There is no message parsing, no slot
//! logic, no protocol metadata, no parent/child lineage, and no multi-board
//! multiplexing — all of which live client-side or in the datalog (§5–§6, §8).
//! The only richness is the S3 two-step upload flow (initiate → upload →
//! confirm), a pure transport detail (§8.1).

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use uuid::Uuid;

use crate::api_types::{
    BoardResponse, BoardsListResponse, ConfirmMessageRequest, ConfirmMessageResponse, ContentType,
    CreateBoardRequest, GetBlobResponse, GetBlobsQuery, GetBlobsResponse, InitiateMessageRequest,
    InitiateMessageResponse, ListBlobsResponse, MessageBlobWithUrl, MAX_INLINE_MESSAGE_SIZE,
};

use crate::{db, s3, state::AppState};

pub async fn create_board(
    State(state): State<AppState>,
    Json(req): Json<CreateBoardRequest>,
) -> Result<Json<BoardResponse>, StatusCode> {
    let board = db::create_board(&state.db, &req.name).await.map_err(|e| {
        tracing::error!("Failed to create board: {}", e);
        StatusCode::BAD_REQUEST
    })?;

    Ok(Json(BoardResponse {
        name: board.name,
        created_at: board.created_at,
        status: board.status,
    }))
}

pub async fn get_board(
    State(state): State<AppState>,
    Path(board_name): Path<String>,
) -> Result<Json<BoardResponse>, StatusCode> {
    let board = db::get_board(&state.db, &board_name)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get board: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(BoardResponse {
        name: board.name,
        created_at: board.created_at,
        status: board.status,
    }))
}

pub async fn list_boards(
    State(state): State<AppState>,
) -> Result<Json<BoardsListResponse>, StatusCode> {
    let boards = db::list_boards(&state.db).await.map_err(|e| {
        tracing::error!("Failed to list boards: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(BoardsListResponse {
        boards: boards
            .into_iter()
            .map(|b| BoardResponse {
                name: b.name,
                created_at: b.created_at,
                status: b.status,
            })
            .collect(),
    }))
}

/// Step 1 of the S3 two-step flow: reserve a message id and, for a large
/// message, a pre-signed S3 upload URL. Small messages are sent inline in the
/// confirm request.
pub async fn initiate_message(
    State(state): State<AppState>,
    Path(board_name): Path<String>,
    Json(req): Json<InitiateMessageRequest>,
) -> Result<Json<InitiateMessageResponse>, StatusCode> {
    db::get_board(&state.db, &board_name)
        .await
        .map_err(|e| {
            tracing::error!("Failed to check board: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or_else(|| {
            tracing::error!("Board not found: {}", board_name);
            StatusCode::NOT_FOUND
        })?;

    let message_id = Uuid::new_v4().to_string();

    if req.size > MAX_INLINE_MESSAGE_SIZE {
        // Large message - generate S3 upload URL.
        let s3_key = format!("{}/messages/{}", board_name, message_id);
        let upload_url = s3::generate_upload_url(&state.s3_client, &state.bucket_name, &s3_key)
            .await
            .map_err(|e| {
                tracing::error!("[S3] Failed to generate upload URL: {}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?;

        Ok(Json(InitiateMessageResponse {
            message_id,
            upload_url: Some(upload_url),
            should_upload: true,
        }))
    } else {
        // Small message - client sends data in the confirm request.
        Ok(Json(InitiateMessageResponse {
            message_id,
            upload_url: None,
            should_upload: false,
        }))
    }
}

/// Step 2 of the S3 two-step flow: record the message. b4 stores the bytes
/// verbatim (inline) or records the S3 key (already uploaded by the client). It
/// does NOT deserialize or interpret the message — hence no `Context` generic.
pub async fn confirm_message(
    State(state): State<AppState>,
    Path((board_name, s3_message_id)): Path<(String, String)>,
    Json(req): Json<ConfirmMessageRequest>,
) -> Result<Json<ConfirmMessageResponse>, StatusCode> {
    db::get_board(&state.db, &board_name)
        .await
        .map_err(|e| {
            tracing::error!("Failed to check board: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or_else(|| {
            tracing::error!("Board not found: {}", board_name);
            StatusCode::NOT_FOUND
        })?;

    let version = req.version;

    if let Some(data) = req.data {
        // Inline message - store the opaque bytes as-is.
        db::insert_message(
            &state.db,
            &board_name,
            Some(data.as_slice()),
            None,
            &version,
        )
        .await
        .map_err(|e| {
            tracing::error!("Failed to insert inline message: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    } else {
        // S3 message - the client has already uploaded to this key; just record
        // it (b4 never downloads or inspects the object).
        let s3_key = format!("{}/messages/{}", board_name, s3_message_id);
        db::insert_message(
            &state.db,
            &board_name,
            None,
            Some(s3_key.as_str()),
            &version,
        )
        .await
        .map_err(|e| {
            tracing::error!("Failed to insert S3 message: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    }

    tracing::info!(
        "confirm_message: recorded one message on board '{}' (id {})",
        board_name,
        s3_message_id
    );
    Ok(Json(ConfirmMessageResponse { success: true }))
}

pub async fn get_message(
    State(state): State<AppState>,
    Path((board_name, id)): Path<(String, String)>,
) -> Result<Json<GetBlobResponse>, StatusCode> {
    let id_num: i64 = id.parse().map_err(|_| {
        tracing::error!("Invalid message ID: {}", id);
        StatusCode::BAD_REQUEST
    })?;

    let message = db::get_message(&state.db, &board_name, id_num)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get message: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?;

    let download_url = s3_download_url(&state, &message.content_type).await?;

    Ok(Json(GetBlobResponse {
        message,
        download_url,
    }))
}

pub async fn list_messages(
    State(state): State<AppState>,
    Path(board_name): Path<String>,
    Query(query): Query<GetBlobsQuery>,
) -> Result<Json<ListBlobsResponse>, StatusCode> {
    let messages = fetch_board_messages(&state, &board_name, &query).await?;
    Ok(Json(ListBlobsResponse { messages }))
}

pub async fn get_messages(
    State(state): State<AppState>,
    Path(board_name): Path<String>,
    Query(query): Query<GetBlobsQuery>,
) -> Result<Json<GetBlobsResponse>, StatusCode> {
    let messages = fetch_board_messages(&state, &board_name, &query).await?;

    let mut enriched = Vec::with_capacity(messages.len());
    for message in messages {
        let download_url = s3_download_url(&state, &message.content_type).await?;
        enriched.push(MessageBlobWithUrl {
            message,
            download_url,
        });
    }

    tracing::info!("get_messages: returning {} messages", enriched.len());
    Ok(Json(GetBlobsResponse { messages: enriched }))
}

/// Fetch a board's messages, honouring the optional `last_id` incremental cursor
/// (§8.5/§12) — v0.6 clients omit it and get the full board.
async fn fetch_board_messages(
    state: &AppState,
    board_name: &str,
    query: &GetBlobsQuery,
) -> Result<Vec<crate::api_types::MessageBlob>, StatusCode> {
    if let Some(last_id) = query.last_id {
        let limit = query.limit.unwrap_or(100).min(1000);
        let (messages, _truncated) = db::get_messages_after(&state.db, board_name, last_id, limit)
            .await
            .map_err(|e| {
                tracing::error!("Failed to get messages after ID: {}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?;
        Ok(messages)
    } else {
        db::list_messages(&state.db, board_name).await.map_err(|e| {
            tracing::error!("Failed to list messages: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })
    }
}

/// A pre-signed download URL for an S3-backed message, or `None` for inline.
async fn s3_download_url(
    state: &AppState,
    content_type: &ContentType,
) -> Result<Option<String>, StatusCode> {
    match content_type {
        ContentType::S3 { key } => {
            let url = s3::generate_download_url(&state.s3_client, &state.bucket_name, key)
                .await
                .map_err(|e| {
                    tracing::error!("[S3] Failed to generate download URL: {}", e);
                    StatusCode::INTERNAL_SERVER_ERROR
                })?;
            Ok(Some(url))
        }
        ContentType::Inline { .. } => Ok(None),
    }
}
