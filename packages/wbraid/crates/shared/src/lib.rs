use serde::{Deserialize, Serialize};

/// Maximum size for inline message storage (set to 0 to force all messages to S3 for testing)
pub const MAX_INLINE_MESSAGE_SIZE: usize = 0; // Was: 1024 * 1024 (1MB)

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: String,
    pub timestamp: i64,
    pub content_type: ContentType,
    pub size: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ContentType {
    Inline { data: Vec<u8> },
    S3 { key: String },
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InitiateMessageRequest {
    pub size: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InitiateMessageResponse {
    pub message_id: String,
    pub upload_url: Option<String>,
    pub should_upload: bool, // true if client should upload to S3, false if sending inline data
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ConfirmMessageRequest {
    pub data: Option<Vec<u8>>, // Only for inline messages
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ConfirmMessageResponse {
    pub success: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GetMessageResponse {
    pub message: Message,
    pub download_url: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ListMessagesResponse {
    pub messages: Vec<Message>,
}

// B3 message types (extracted from b3 crate for shared use across native and WASM)
pub mod b3_messages;

