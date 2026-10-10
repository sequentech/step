// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::api_types::{
    BoardMessagesResponse, ConfirmMessageRequest, ConfirmMessageResponse,
    ConfirmMessagesMultiRequest, ConfirmMessagesMultiResponse, ContentType, GetMessageResponse,
    GetMessagesMultiRequest, GetMessagesMultiResponse, InitiateMessageRequest,
    InitiateMessageResponse, InitiateMessagesMultiRequest, InitiateMessagesMultiResponse,
    ListMessagesResponse, Message, MAX_INLINE_MESSAGE_SIZE,
};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use strand::{backend::ristretto::RistrettoCtx, serialization::StrandDeserialize};
use uuid::Uuid;

use crate::messages::{
    artifact::Configuration, message::Message as BoardMessage, statement::StatementType,
    trusted_board,
};
use crate::{db, s3, state::AppState};

#[derive(Debug, Serialize)]
pub struct BoardResponse {
    pub name: String,
    pub created_at: i64,
    pub status: String,
}

#[derive(Debug, Serialize)]
pub struct BoardsListResponse {
    pub boards: Vec<BoardResponse>,
}

#[derive(Debug, Deserialize)]
pub struct CreateBoardRequest {
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct GetMessagesQuery {
    pub last_id: Option<i64>,
    pub limit: Option<i64>,
}

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

pub async fn initiate_message(
    State(state): State<AppState>,
    Path(board_name): Path<String>,
    Json(req): Json<InitiateMessageRequest>,
) -> Result<Json<InitiateMessageResponse>, StatusCode> {
    // Validate board exists
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
    let size = req.size;

    if size > MAX_INLINE_MESSAGE_SIZE {
        // Large message - generate S3 upload URL
        let s3_key = format!("{}/messages/{}", board_name, message_id);

        tracing::debug!(
            "[S3] Generating upload URL for s3://{}/{}",
            state.bucket_name,
            s3_key
        );
        let upload_url = s3::generate_upload_url(&state.s3_client, &state.bucket_name, &s3_key)
            .await
            .map_err(|e| {
                tracing::error!("[S3] Failed to generate upload URL: {}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?;
        tracing::debug!(
            "[S3] Generated upload URL for board '{}' message {}",
            board_name,
            message_id
        );

        Ok(Json(InitiateMessageResponse {
            message_id,
            upload_url: Some(upload_url),
            should_upload: true,
        }))
    } else {
        // Small message - client should send data in confirm request
        Ok(Json(InitiateMessageResponse {
            message_id,
            upload_url: None,
            should_upload: false,
        }))
    }
}

/// The configuration of a board, which is provisioned through the database and
/// is the only trust anchor for the messages posted to it.
async fn board_configuration(
    state: &AppState,
    board_name: &str,
) -> Result<Configuration<RistrettoCtx>, StatusCode> {
    let rows = db::get_configuration_messages(&state.db, board_name)
        .await
        .map_err(|e| {
            tracing::error!(
                "Failed to load the configuration of board '{}': {}",
                board_name,
                e
            );
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    let messages = rows
        .iter()
        .map(|row| BoardMessage::strand_deserialize(row))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| {
            tracing::error!("Invalid configuration on board '{}': {}", board_name, e);
            StatusCode::PRECONDITION_FAILED
        })?;
    let [provisioned] = messages.as_slice() else {
        tracing::error!(
            "Board '{}' must have one provisioned configuration",
            board_name
        );
        return Err(StatusCode::PRECONDITION_FAILED);
    };
    trusted_board::verify_board::<RistrettoCtx>(&messages, &provisioned.sender.pk).map_err(|e| {
        tracing::error!("Invalid configuration on board '{}': {}", board_name, e);
        StatusCode::PRECONDITION_FAILED
    })
}

/// Downloads a message that was uploaded to S3.
async fn download_message(state: &AppState, s3_key: &str) -> Result<Vec<u8>, StatusCode> {
    tracing::debug!("[S3] GET s3://{}/{}", state.bucket_name, s3_key);
    let object = state
        .s3_client
        .get_object()
        .bucket(&state.bucket_name)
        .key(s3_key)
        .send()
        .await
        .map_err(|e| {
            tracing::error!("[S3] Failed to GET object {}: {}", s3_key, e);
            StatusCode::BAD_REQUEST
        })?;
    let bytes = object.body.collect().await.map_err(|e| {
        tracing::error!("[S3] Failed to read object body {}: {}", s3_key, e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    Ok(bytes.to_vec())
}

/// Verifies a posted message against the board configuration and describes it
/// from its own contents rather than from what the request claims.
fn verified_record(
    board_name: &str,
    message_id: &str,
    timestamp: i64,
    data: &[u8],
    s3_key: Option<&str>,
    configuration: &Configuration<RistrettoCtx>,
) -> Result<Message, StatusCode> {
    let parsed = BoardMessage::strand_deserialize(data).map_err(|e| {
        tracing::error!(
            "Failed to deserialize message for board '{}': {}",
            board_name,
            e
        );
        StatusCode::BAD_REQUEST
    })?;
    if parsed.statement.get_kind() == StatementType::Configuration {
        tracing::warn!(
            "Rejected configuration message for board '{}': it is provisioned separately",
            board_name
        );
        return Err(StatusCode::FORBIDDEN);
    }
    trusted_board::verify_message(&parsed, configuration).map_err(|e| {
        tracing::warn!("Rejected message for board '{}': {}", board_name, e);
        StatusCode::FORBIDDEN
    })?;

    let sender_pk = parsed.sender.pk.to_der_b64_string().map_err(|e| {
        tracing::error!(
            "Failed to encode sender_pk for board '{}': {}",
            board_name,
            e
        );
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    let statement_kind = format!("{:?}", parsed.statement.get_kind());
    let batch: i32 = parsed
        .statement
        .get_batch_number()
        .try_into()
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    let mix_number: i32 = parsed
        .statement
        .get_mix_number()
        .try_into()
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    let content_type = match s3_key {
        Some(key) => ContentType::S3 {
            key: key.to_string(),
        },
        None => ContentType::Inline {
            data: data.to_vec(),
        },
    };

    Ok(Message {
        id: message_id.to_string(),
        timestamp,
        size: data.len(),
        content_type,
        sender_pk,
        statement_kind,
        batch,
        mix_number,
    })
}

async fn store_message(
    state: &AppState,
    board_name: &str,
    message: &Message,
) -> Result<(), StatusCode> {
    let (inline_data, s3_key) = match &message.content_type {
        ContentType::Inline { data } => (Some(data.as_slice()), None),
        ContentType::S3 { key } => (None, Some(key.as_str())),
    };
    db::insert_message(
        &state.db,
        board_name,
        message,
        inline_data,
        s3_key,
        &crate::get_schema_version(),
        &message.sender_pk,
        &message.statement_kind,
        message.batch,
        message.mix_number,
    )
    .await
    .map(|_| ())
    .map_err(|e| {
        tracing::error!("Failed to insert message: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })
}

pub async fn confirm_message(
    State(state): State<AppState>,
    Path((board_name, id)): Path<(String, String)>,
    Json(req): Json<ConfirmMessageRequest>,
) -> Result<Json<ConfirmMessageResponse>, StatusCode> {
    // Validate board exists
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

    let configuration = board_configuration(&state, &board_name).await?;
    let timestamp = Utc::now().timestamp();

    // Check if this is an S3 message or inline message
    let (data, s3_key) = match req.data {
        Some(data) => (data, None),
        None => {
            let s3_key = format!("{}/messages/{}", board_name, id);
            (download_message(&state, &s3_key).await?, Some(s3_key))
        }
    };
    let message = verified_record(
        &board_name,
        &id,
        timestamp,
        &data,
        s3_key.as_deref(),
        &configuration,
    )?;
    store_message(&state, &board_name, &message).await?;

    Ok(Json(ConfirmMessageResponse { success: true }))
}

pub async fn get_message(
    State(state): State<AppState>,
    Path((board_name, id)): Path<(String, String)>,
) -> Result<Json<GetMessageResponse>, StatusCode> {
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

    let download_url = match &message.content_type {
        ContentType::S3 { key } => {
            tracing::debug!(
                "[S3] Generating download URL for s3://{}/{}",
                state.bucket_name,
                key
            );
            Some(
                s3::generate_download_url(&state.s3_client, &state.bucket_name, key)
                    .await
                    .map_err(|e| {
                        tracing::error!("[S3] Failed to generate download URL: {}", e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })?,
            )
        }
        ContentType::Inline { .. } => None,
    };

    Ok(Json(GetMessageResponse {
        message,
        download_url,
    }))
}

pub async fn list_messages(
    State(state): State<AppState>,
    Path(board_name): Path<String>,
    Query(query): Query<GetMessagesQuery>,
) -> Result<Json<ListMessagesResponse>, StatusCode> {
    // If last_id is provided, use range query
    if let Some(last_id) = query.last_id {
        let limit = query.limit.unwrap_or(100).min(1000); // Max 1000 messages per request

        let (messages, _truncated) = db::get_messages_after(&state.db, &board_name, last_id, limit)
            .await
            .map_err(|e| {
                tracing::error!("Failed to get messages after ID: {}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?;

        if messages.is_empty() {
            tracing::warn!(
                "Found 0 messages for list_messages request on board '{}' with last_id {}",
                board_name,
                last_id
            );
        }

        // TODO: Return truncated flag in response for pagination
        Ok(Json(ListMessagesResponse { messages }))
    } else {
        // Get all messages for the board
        let messages = db::list_messages(&state.db, &board_name)
            .await
            .map_err(|e| {
                tracing::error!("Failed to list messages: {}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?;

        Ok(Json(ListMessagesResponse { messages }))
    }
}

pub async fn get_messages(
    State(state): State<AppState>,
    Path(board_name): Path<String>,
    Query(query): Query<GetMessagesQuery>,
) -> Result<Json<crate::api_types::GetMessagesResponse>, StatusCode> {
    use crate::api_types::{GetMessagesResponse, MessageWithUrl};

    // Get messages using same logic as list_messages
    let messages = if let Some(last_id) = query.last_id {
        let limit = query.limit.unwrap_or(100).min(1000);

        let (msgs, _truncated) = db::get_messages_after(&state.db, &board_name, last_id, limit)
            .await
            .map_err(|e| {
                tracing::error!("Failed to get messages after ID: {}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?;

        if msgs.is_empty() {
            tracing::warn!(
                "Found 0 messages for get_messages request on board '{}' with last_id {}",
                board_name,
                last_id
            );
        }

        msgs
    } else {
        db::list_messages(&state.db, &board_name)
            .await
            .map_err(|e| {
                tracing::error!("Failed to list messages: {}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?
    };

    // Generate download URLs for S3 messages
    let mut enriched_messages = Vec::new();
    for msg in messages {
        let download_url = match &msg.content_type {
            ContentType::S3 { key } => {
                tracing::debug!(
                    "[S3] Generating download URL for s3://{}/{}",
                    state.bucket_name,
                    key
                );
                Some(
                    s3::generate_download_url(&state.s3_client, &state.bucket_name, key)
                        .await
                        .map_err(|e| {
                            tracing::error!("[S3] Failed to generate download URL: {}", e);
                            StatusCode::INTERNAL_SERVER_ERROR
                        })?,
                )
            }
            ContentType::Inline { .. } => None,
        };

        enriched_messages.push(MessageWithUrl {
            message: msg,
            download_url,
        });
    }

    tracing::debug!(
        "get_messages: returning {} messages with download URLs",
        enriched_messages.len()
    );
    Ok(Json(GetMessagesResponse {
        messages: enriched_messages,
    }))
}

pub async fn get_messages_multi(
    State(state): State<AppState>,
    Json(req): Json<GetMessagesMultiRequest>,
) -> Result<Json<GetMessagesMultiResponse>, StatusCode> {
    use crate::api_types::MessageWithUrl;

    tracing::info!(
        "[MULTI-GET] {} boards in single request",
        req.requests.len()
    );

    let mut boards = Vec::new();

    for board_req in req.requests {
        let last_id = board_req.last_id;
        let limit = board_req.limit.unwrap_or(100).min(1000); // Default 100, max 1000

        let (messages, has_more) =
            db::get_messages_after(&state.db, &board_req.board, last_id, limit)
                .await
                .map_err(|e| {
                    tracing::error!(
                        "Failed to get messages for board {}: {}",
                        board_req.board,
                        e
                    );
                    StatusCode::INTERNAL_SERVER_ERROR
                })?;

        // Generate download URLs for S3 messages
        let mut enriched_messages = Vec::new();
        for msg in messages {
            let download_url = match &msg.content_type {
                ContentType::S3 { key } => {
                    tracing::debug!(
                        "[S3] Generating download URL for s3://{}/{}",
                        state.bucket_name,
                        key
                    );
                    Some(
                        s3::generate_download_url(&state.s3_client, &state.bucket_name, key)
                            .await
                            .map_err(|e| {
                                tracing::error!("[S3] Failed to generate download URL: {}", e);
                                StatusCode::INTERNAL_SERVER_ERROR
                            })?,
                    )
                }
                ContentType::Inline { .. } => None,
            };

            enriched_messages.push(MessageWithUrl {
                message: msg,
                download_url,
            });
        }

        tracing::info!(
            "  -> Board '{}': last_id={}, limit={}, returned={} messages{}",
            board_req.board,
            last_id,
            limit,
            enriched_messages.len(),
            if has_more {
                " (paginated, more available)"
            } else {
                ""
            }
        );

        boards.push(BoardMessagesResponse {
            board: board_req.board,
            messages: enriched_messages,
        });
    }

    tracing::info!("[MULTI-GET] Complete: {} boards processed", boards.len());
    Ok(Json(GetMessagesMultiResponse { boards }))
}

// Multi-board S3 two-step flow handlers

pub async fn initiate_messages_multi(
    State(state): State<AppState>,
    Json(req): Json<InitiateMessagesMultiRequest>,
) -> Result<Json<InitiateMessagesMultiResponse>, StatusCode> {
    use crate::api_types::{BoardInitiateResponse, MessageUploadInfo};

    let mut board_responses = Vec::new();

    for board_req in req.requests {
        let board_name = &board_req.board;

        // Validate board exists
        db::get_board(&state.db, board_name)
            .await
            .map_err(|e| {
                tracing::error!("Failed to check board '{}': {}", board_name, e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?
            .ok_or_else(|| {
                tracing::error!("Board not found: {}", board_name);
                StatusCode::NOT_FOUND
            })?;

        let mut uploads = Vec::new();

        for msg_meta in board_req.messages {
            let message_id = Uuid::new_v4().to_string();
            let size = msg_meta.size;

            if size > MAX_INLINE_MESSAGE_SIZE {
                // Large message - generate S3 upload URL
                let s3_key = format!("{}/messages/{}", board_name, message_id);

                tracing::debug!(
                    "[S3] Generating upload URL for s3://{}/{}",
                    state.bucket_name,
                    s3_key
                );
                let upload_url =
                    s3::generate_upload_url(&state.s3_client, &state.bucket_name, &s3_key)
                        .await
                        .map_err(|e| {
                            tracing::error!(
                                "[S3] Failed to generate upload URL for board '{}': {}",
                                board_name,
                                e
                            );
                            StatusCode::INTERNAL_SERVER_ERROR
                        })?;
                tracing::debug!("[S3] Generated upload URL for board '{}'", board_name);

                uploads.push(MessageUploadInfo {
                    message_id,
                    upload_url: Some(upload_url),
                    should_upload: true,
                });
            } else {
                // Small message - client should send data in confirm request
                uploads.push(MessageUploadInfo {
                    message_id,
                    upload_url: None,
                    should_upload: false,
                });
            }
        }

        board_responses.push(BoardInitiateResponse {
            board: board_name.clone(),
            uploads,
        });
    }

    tracing::info!(
        "[MULTI-INITIATE] Prepared {} boards for upload",
        board_responses.len()
    );

    Ok(Json(InitiateMessagesMultiResponse {
        boards: board_responses,
    }))
}

pub async fn confirm_messages_multi(
    State(state): State<AppState>,
    Json(req): Json<ConfirmMessagesMultiRequest>,
) -> Result<Json<ConfirmMessagesMultiResponse>, StatusCode> {
    use crate::api_types::ConfirmMessagesMultiResponse;

    let board_count = req.requests.len();
    tracing::info!(
        "[MULTI-CONFIRM] Confirming messages for {} boards",
        board_count
    );

    // Every message of the request is verified before any of them is stored.
    let mut verified: Vec<(String, Message)> = Vec::new();
    for board_req in req.requests {
        let board_name = &board_req.board;

        // Validate board exists
        db::get_board(&state.db, board_name)
            .await
            .map_err(|e| {
                tracing::error!("Failed to check board '{}': {}", board_name, e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?
            .ok_or_else(|| {
                tracing::error!("Board not found: {}", board_name);
                StatusCode::NOT_FOUND
            })?;

        let configuration = board_configuration(&state, board_name).await?;
        let timestamp = Utc::now().timestamp();

        let mut inline_count = 0;
        let mut s3_count = 0;
        let confirmation_count = board_req.confirmations.len();

        for confirmation in board_req.confirmations {
            let message_id = &confirmation.message_id;

            let (data, s3_key) = match confirmation.data {
                Some(data) => {
                    inline_count += 1;
                    (data, None)
                }
                None => {
                    // S3 message - download to verify it
                    s3_count += 1;
                    let s3_key = format!("{}/messages/{}", board_name, message_id);
                    (download_message(&state, &s3_key).await?, Some(s3_key))
                }
            };
            let message = verified_record(
                board_name,
                message_id,
                timestamp,
                &data,
                s3_key.as_deref(),
                &configuration,
            )?;
            verified.push((board_name.clone(), message));
        }

        tracing::info!(
            "  -> Board '{}': verified {} messages (inline: {}, S3: {})",
            board_name,
            confirmation_count,
            inline_count,
            s3_count
        );
    }

    for (board_name, message) in &verified {
        store_message(&state, board_name, message).await?;
    }

    tracing::info!("[MULTI-CONFIRM] Complete: {} boards processed", board_count);

    Ok(Json(ConfirmMessagesMultiResponse { success: true }))
}
