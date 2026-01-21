// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! HTTP API types for the bulletin board service
//!
//! This module defines all request/response types used in the HTTP API,
//! including message handling, board operations, and S3 integration.

use base64::Engine;
use serde::{Deserialize, Serialize};

/// Maximum size for inline message storage (set to 0 to force all messages to S3 for testing)
pub const MAX_INLINE_MESSAGE_SIZE: usize = 1024 * 1024; // 1MB

/// A message stored in the bulletin board
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: String,
    pub timestamp: i64,
    pub content_type: ContentType,
    pub size: usize,
    pub sender_pk: String,
    pub statement_kind: String,
    pub batch: i32,
    pub mix_number: i32,
}

/// Content storage type for messages
#[derive(Debug, Clone)]
pub enum ContentType {
    /// Message data stored inline in the database
    Inline { data: Vec<u8> },
    /// Message data stored in S3
    S3 { key: String },
}

// Custom serialization for ContentType to match API expectations
impl Serialize for ContentType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        match self {
            ContentType::Inline { data } => {
                let mut state = serializer.serialize_struct("ContentType", 1)?;
                let encoded = base64::prelude::BASE64_STANDARD.encode(data);
                state.serialize_field("message", &encoded)?;
                state.end()
            }
            ContentType::S3 { key } => {
                let mut state = serializer.serialize_struct("ContentType", 1)?;
                state.serialize_field("key", key)?;
                state.end()
            }
        }
    }
}

impl<'de> Deserialize<'de> for ContentType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum ContentTypeHelper {
            Inline { message: String },
            S3 { key: String },
        }

        let helper = ContentTypeHelper::deserialize(deserializer)?;
        match helper {
            ContentTypeHelper::Inline { message } => {
                let data = base64::prelude::BASE64_STANDARD
                    .decode(&message)
                    .map_err(serde::de::Error::custom)?;
                Ok(ContentType::Inline { data })
            }
            ContentTypeHelper::S3 { key } => Ok(ContentType::S3 { key }),
        }
    }
}

// ============================================================================
// Single Message API Types
// ============================================================================

/// Request to initiate a message upload (step 1 of 2-step S3 flow)
#[derive(Debug, Serialize, Deserialize)]
pub struct InitiateMessageRequest {
    pub size: usize,
    pub sender_pk: String,
    pub statement_kind: String,
    pub batch: i32,
    pub mix_number: i32,
}

/// Response from initiating a message upload
#[derive(Debug, Serialize, Deserialize)]
pub struct InitiateMessageResponse {
    pub message_id: String,
    pub upload_url: Option<String>,
    pub should_upload: bool, // true if client should upload to S3, false if sending inline data
}

/// Request to confirm a message upload (step 2 of 2-step flow)
#[derive(Debug, Serialize, Deserialize)]
pub struct ConfirmMessageRequest {
    pub data: Option<Vec<u8>>, // Only for inline messages
    pub sender_pk: String,
    pub statement_kind: String,
    pub batch: i32,
    pub mix_number: i32,
}

/// Response from confirming a message upload
#[derive(Debug, Serialize, Deserialize)]
pub struct ConfirmMessageResponse {
    pub success: bool,
}

/// Response from getting a single message
#[derive(Debug, Serialize, Deserialize)]
pub struct GetMessageResponse {
    pub message: Message,
    pub download_url: Option<String>,
}

/// Response from listing messages (metadata only)
#[derive(Debug, Serialize, Deserialize)]
pub struct ListMessagesResponse {
    pub messages: Vec<Message>,
}

/// Message with pre-signed download URL for S3 content
#[derive(Debug, Serialize, Deserialize)]
pub struct MessageWithUrl {
    #[serde(flatten)]
    pub message: Message,
    pub download_url: Option<String>,
}

/// Response from getting messages (includes download URLs for immediate use)
#[derive(Debug, Serialize, Deserialize)]
pub struct GetMessagesResponse {
    pub messages: Vec<MessageWithUrl>,
}

// ============================================================================
// Multi-Board API Types (GET)
// ============================================================================

/// Request for messages from a single board
#[derive(Debug, Serialize, Deserialize)]
pub struct BoardMessageRequest {
    pub board: String,
    pub last_id: i64,
    pub limit: Option<i64>,
}

/// Request for messages from multiple boards
#[derive(Debug, Serialize, Deserialize)]
pub struct GetMessagesMultiRequest {
    pub requests: Vec<BoardMessageRequest>,
}

/// Response with messages from a single board (includes download URLs)
#[derive(Debug, Serialize, Deserialize)]
pub struct BoardMessagesResponse {
    pub board: String,
    pub messages: Vec<MessageWithUrl>,
}

/// Response with messages from multiple boards
#[derive(Debug, Serialize, Deserialize)]
pub struct GetMessagesMultiResponse {
    pub boards: Vec<BoardMessagesResponse>,
}

// ============================================================================
// Multi-Board API Types (PUT with S3 two-step flow)
// ============================================================================

/// Metadata for a message to be uploaded
#[derive(Debug, Serialize, Deserialize)]
pub struct MessageMetadata {
    pub size: usize,
    pub sender_pk: String,
    pub statement_kind: String,
    pub batch: i32,
    pub mix_number: i32,
}

/// Request to initiate message uploads to a single board
#[derive(Debug, Serialize, Deserialize)]
pub struct BoardInitiateRequest {
    pub board: String,
    pub messages: Vec<MessageMetadata>,
}

/// Request to initiate message uploads to multiple boards
#[derive(Debug, Serialize, Deserialize)]
pub struct InitiateMessagesMultiRequest {
    pub requests: Vec<BoardInitiateRequest>,
}

/// Upload information for a single message
#[derive(Debug, Serialize, Deserialize)]
pub struct MessageUploadInfo {
    pub message_id: String,
    pub upload_url: Option<String>, // S3 pre-signed URL (None for inline messages)
    pub should_upload: bool,        // true if client should upload to S3
}

/// Response from initiating uploads to a single board
#[derive(Debug, Serialize, Deserialize)]
pub struct BoardInitiateResponse {
    pub board: String,
    pub uploads: Vec<MessageUploadInfo>,
}

/// Response from initiating uploads to multiple boards
#[derive(Debug, Serialize, Deserialize)]
pub struct InitiateMessagesMultiResponse {
    pub boards: Vec<BoardInitiateResponse>,
}

// Step 2: Client uploads to S3 (no API call, direct S3 PUT)

/// Confirmation for a single message upload
#[derive(Debug, Serialize, Deserialize)]
pub struct MessageConfirmation {
    pub message_id: String,
    pub data: Option<Vec<u8>>, // Only for inline messages (when should_upload was false)
}

/// Request to confirm uploads to a single board
#[derive(Debug, Serialize, Deserialize)]
pub struct BoardConfirmRequest {
    pub board: String,
    pub confirmations: Vec<MessageConfirmation>,
}

/// Request to confirm uploads to multiple boards
#[derive(Debug, Serialize, Deserialize)]
pub struct ConfirmMessagesMultiRequest {
    pub requests: Vec<BoardConfirmRequest>,
}

/// Response from confirming multi-board uploads
#[derive(Debug, Serialize, Deserialize)]
pub struct ConfirmMessagesMultiResponse {
    pub success: bool,
}
