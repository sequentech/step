use serde::{Deserialize, Serialize};
use base64::Engine;

/// Maximum size for inline message storage (set to 0 to force all messages to S3 for testing)
pub const MAX_INLINE_MESSAGE_SIZE: usize = 0; // Was: 1024 * 1024 (1MB)

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

#[derive(Debug, Clone)]
pub enum ContentType {
    Inline { data: Vec<u8> },
    S3 { key: String },
}

// Custom serialization for ContentType to match test expectations
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

#[derive(Debug, Serialize, Deserialize)]
pub struct InitiateMessageRequest {
    pub size: usize,
    pub sender_pk: String,
    pub statement_kind: String,
    pub batch: i32,
    pub mix_number: i32,
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
    pub sender_pk: String,
    pub statement_kind: String,
    pub batch: i32,
    pub mix_number: i32,
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

// Multi-board request/response types
#[derive(Debug, Serialize, Deserialize)]
pub struct BoardMessageRequest {
    pub board: String,
    pub last_id: i64,
    pub limit: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GetMessagesMultiRequest {
    pub requests: Vec<BoardMessageRequest>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BoardMessagesResponse {
    pub board: String,
    pub messages: Vec<Message>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GetMessagesMultiResponse {
    pub boards: Vec<BoardMessagesResponse>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BoardPutRequest {
    pub board: String,
    pub messages: Vec<Vec<u8>>, // Serialized Message structs
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PutMessagesMultiRequest {
    pub requests: Vec<BoardPutRequest>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PutMessagesMultiResponse {
    pub success: bool,
}

// B3 message types (extracted from b3 crate for shared use across native and WASM)
pub mod b3_messages;

