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
    MAX_INLINE_MESSAGE_SIZE,
};

use crate::{db, s3, state::AppState};

#[derive(Debug, Serialize)]
pub struct BoardResponse {
    pub name: String,
    pub created_at: i64,
    pub status: String,
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
) -> Result<Json<Vec<BoardResponse>>, StatusCode> {
    let boards = db::list_boards(&state.db)
        .await
        .map_err(|e| {
            tracing::error!("Failed to list boards: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Json(boards
        .into_iter()
        .map(|b| BoardResponse {
            name: b.name,
            created_at: b.created_at,
            status: b.status,
        })
        .collect()))
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
