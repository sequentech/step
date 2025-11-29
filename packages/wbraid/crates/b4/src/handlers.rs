use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use wbraid_shared::{
    ContentType, GetMessageResponse, ListMessagesResponse, Message,
    InitiateMessageRequest, InitiateMessageResponse, ConfirmMessageRequest, ConfirmMessageResponse,
    GetMessagesMultiRequest, GetMessagesMultiResponse, BoardMessagesResponse,
    PutMessagesMultiRequest, PutMessagesMultiResponse,
    MAX_INLINE_MESSAGE_SIZE,
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
    let board = db::create_board(&state.db, &req.name)
        .await
        .map_err(|e| {
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
    let boards = db::list_boards(&state.db)
        .await
        .map_err(|e| {
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
        
        let upload_url = s3::generate_upload_url(&state.s3_client, &state.bucket_name, &s3_key)
            .await
            .map_err(|e| {
                tracing::error!("Failed to generate upload URL: {}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?;

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
    
    let timestamp = Utc::now().timestamp();
    let version = "1".to_string(); // Use schema version

    // Check if this is an S3 message or inline message
    if let Some(data) = req.data {
        // Inline message
        let size = data.len();
        let content_type = ContentType::Inline { data: data.clone() };
        
        let msg = Message {
            id: id.clone(),
            timestamp,
            size,
            content_type,
            sender_pk: req.sender_pk.clone(),
            statement_kind: req.statement_kind.clone(),
            batch: req.batch,
            mix_number: req.mix_number,
        };

        db::insert_message(
            &state.db,
            &board_name,
            &msg,
            Some(data.as_slice()),
            None,
            &version,
            &req.sender_pk,
            &req.statement_kind,
            req.batch,
            req.mix_number,
        )
            .await
            .map_err(|e| {
                tracing::error!("Failed to insert message: {}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?;

    } else {
        // S3 message - verify upload and get size
        let s3_key = format!("{}/messages/{}", board_name, id);
        
        // Get object metadata from S3 to determine size
        let size = match state.s3_client
            .head_object()
            .bucket(&state.bucket_name)
            .key(&s3_key)
            .send()
            .await
        {
            Ok(output) => output.content_length().unwrap_or(0) as usize,
            Err(e) => {
                tracing::error!("Failed to get S3 object metadata: {}", e);
                return Err(StatusCode::BAD_REQUEST);
            }
        };

        let content_type = ContentType::S3 { key: s3_key.clone() };
        
        let msg = Message {
            id: id.clone(),
            timestamp,
            size,
            content_type,
            sender_pk: req.sender_pk.clone(),
            statement_kind: req.statement_kind.clone(),
            batch: req.batch,
            mix_number: req.mix_number,
        };

        db::insert_message(
            &state.db,
            &board_name,
            &msg,
            None,
            Some(s3_key.as_str()),
            &version,
            &req.sender_pk,
            &req.statement_kind,
            req.batch,
            req.mix_number,
        )
            .await
            .map_err(|e| {
                tracing::error!("Failed to insert message: {}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?;

    }

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
            Some(s3::generate_download_url(&state.s3_client, &state.bucket_name, key)
                .await
                .map_err(|e| {
                    tracing::error!("Failed to generate download URL: {}", e);
                    StatusCode::INTERNAL_SERVER_ERROR
                })?)
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

pub async fn get_messages_multi(
    State(state): State<AppState>,
    Json(req): Json<GetMessagesMultiRequest>,
) -> Result<Json<GetMessagesMultiResponse>, StatusCode> {
    tracing::info!(
        "[MULTI-GET] {} boards in single request",
        req.requests.len()
    );
    
    let mut boards = Vec::new();
    
    for board_req in req.requests {
        let last_id = board_req.last_id;
        let limit = board_req.limit.unwrap_or(100).min(1000); // Default 100, max 1000
        
        let (messages, has_more) = db::get_messages_after(&state.db, &board_req.board, last_id, limit)
            .await
            .map_err(|e| {
                tracing::error!("Failed to get messages for board {}: {}", board_req.board, e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?;
        
        tracing::info!(
            "  -> Board '{}': last_id={}, limit={}, returned={} messages{}",
            board_req.board,
            last_id,
            limit,
            messages.len(),
            if has_more { " (paginated, more available)" } else { "" }
        );
        
        boards.push(BoardMessagesResponse {
            board: board_req.board,
            messages,
        });
    }
    
    tracing::info!("[MULTI-GET] Complete: {} boards processed", boards.len());
    Ok(Json(GetMessagesMultiResponse { boards }))
}

pub async fn put_messages_multi(
    State(state): State<AppState>,
    Json(req): Json<PutMessagesMultiRequest>,
) -> Result<Json<PutMessagesMultiResponse>, StatusCode> {
    use b4::messages::message::Message as B4Message;
    use strand::serialization::StrandDeserialize;
    
    let total_messages: usize = req.requests.iter().map(|r| r.messages.len()).sum();
    let board_count = req.requests.len();
    tracing::info!(
        "[MULTI-PUT] {} boards, {} total messages in single request",
        board_count,
        total_messages
    );
    
    for board_req in req.requests {
        tracing::info!(
            "  -> Board '{}': processing {} messages",
            board_req.board,
            board_req.messages.len()
        );
        
        // Validate board exists
        db::get_board(&state.db, &board_req.board)
            .await
            .map_err(|e| {
                tracing::error!("Failed to check board: {}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?
            .ok_or_else(|| {
                tracing::error!("Board not found: {}", board_req.board);
                StatusCode::NOT_FOUND
            })?;
        
        let mut s3_count = 0;
        let mut inline_count = 0;
        
        for message_bytes in board_req.messages {
            // Deserialize the B4 Message
            let message = B4Message::strand_deserialize(&message_bytes)
                .map_err(|e| {
                    tracing::error!("Failed to deserialize message: {}", e);
                    StatusCode::BAD_REQUEST
                })?;
            
            // Extract metadata
            let sender_pk = message.sender.pk.to_der_b64_string()
                .map_err(|e| {
                    tracing::error!("Failed to encode sender pk: {}", e);
                    StatusCode::INTERNAL_SERVER_ERROR
                })?;
            let statement_kind = message.statement.get_kind().to_string();
            let batch: i32 = message.statement.get_batch_number().try_into()
                .map_err(|_| StatusCode::BAD_REQUEST)?;
            let mix_number: i32 = message.statement.get_mix_number().try_into()
                .map_err(|_| StatusCode::BAD_REQUEST)?;
            
            let timestamp = Utc::now().timestamp();
            let version = "1".to_string();
            let size = message_bytes.len();
            
            // Determine if inline or S3
            if size > MAX_INLINE_MESSAGE_SIZE {
                // S3 upload
                let message_id = Uuid::new_v4().to_string();
                let s3_key = format!("{}/messages/{}", board_req.board, message_id);
                
                // Upload to S3
                state.s3_client
                    .put_object()
                    .bucket(&state.bucket_name)
                    .key(&s3_key)
                    .body(message_bytes.clone().into())
                    .send()
                    .await
                    .map_err(|e| {
                        tracing::error!("Failed to upload to S3: {}", e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })?;
                
                let content_type = ContentType::S3 { key: s3_key.clone() };
                let msg = wbraid_shared::Message {
                    id: message_id.clone(),
                    timestamp,
                    size,
                    content_type,
                    sender_pk: sender_pk.clone(),
                    statement_kind: statement_kind.clone(),
                    batch,
                    mix_number,
                };
                
                db::insert_message(
                    &state.db,
                    &board_req.board,
                    &msg,
                    None,
                    Some(s3_key.as_str()),
                    &version,
                    &sender_pk,
                    &statement_kind,
                    batch,
                    mix_number,
                )
                    .await
                    .map_err(|e| {
                        tracing::error!("Failed to insert S3 message: {}", e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })?;
                
                s3_count += 1;
            } else {
                // Inline storage
                let message_id = Uuid::new_v4().to_string();
                let content_type = ContentType::Inline { data: message_bytes.clone() };
                
                let msg = wbraid_shared::Message {
                    id: message_id.clone(),
                    timestamp,
                    size,
                    content_type,
                    sender_pk: sender_pk.clone(),
                    statement_kind: statement_kind.clone(),
                    batch,
                    mix_number,
                };
                
                db::insert_message(
                    &state.db,
                    &board_req.board,
                    &msg,
                    Some(message_bytes.as_slice()),
                    None,
                    &version,
                    &sender_pk,
                    &statement_kind,
                    batch,
                    mix_number,
                )
                    .await
                    .map_err(|e| {
                        tracing::error!("Failed to insert inline message: {}", e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })?;
                
                inline_count += 1;
            }
        }
        
        tracing::info!(
            "     Board '{}': stored {} messages (inline: {}, S3: {})",
            board_req.board,
            inline_count + s3_count,
            inline_count,
            s3_count
        );
    }
    
    tracing::info!("[MULTI-PUT] Complete: {} boards processed", board_count);
    Ok(Json(PutMessagesMultiResponse { success: true }))
}

