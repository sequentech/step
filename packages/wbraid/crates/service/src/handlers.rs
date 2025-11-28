use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use chrono::Utc;
use uuid::Uuid;
use wbraid_shared::{
    ContentType, GetMessageResponse, ListMessagesResponse, Message,
    InitiateMessageRequest, InitiateMessageResponse, ConfirmMessageRequest, ConfirmMessageResponse,
    MAX_INLINE_MESSAGE_SIZE,
};

use crate::{db, s3, state::AppState};

pub async fn initiate_message(
    State(state): State<AppState>,
    Json(req): Json<InitiateMessageRequest>,
) -> Result<Json<InitiateMessageResponse>, StatusCode> {
    let message_id = Uuid::new_v4().to_string();
    let size = req.size;

    if size > MAX_INLINE_MESSAGE_SIZE {
        // Large message - generate S3 upload URL
        let s3_key = format!("messages/{}", message_id);
        
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
    Path(id): Path<String>,
    Json(req): Json<ConfirmMessageRequest>,
) -> Result<Json<ConfirmMessageResponse>, StatusCode> {
    let timestamp = Utc::now().timestamp();

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
        };

        db::insert_message(&state.db, &msg, Some(data.as_slice()), None)
            .await
            .map_err(|e| {
                tracing::error!("Failed to insert message: {}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?;

    } else {
        // S3 message - verify upload and get size
        let s3_key = format!("messages/{}", id);
        
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
        };

        db::insert_message(&state.db, &msg, None, Some(s3_key.as_str()))
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
    Path(id): Path<String>,
) -> Result<Json<GetMessageResponse>, StatusCode> {
    let message = db::get_message(&state.db, &id)
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
) -> Result<Json<ListMessagesResponse>, StatusCode> {
    let messages = db::list_messages(&state.db)
        .await
        .map_err(|e| {
            tracing::error!("Failed to list messages: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Json(ListMessagesResponse { messages }))
}
