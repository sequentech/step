// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! HTTP API types for the bulletin board service.
//!
//! This module defines all request/response types used in the HTTP API,
//! including blob storage, board operations, and S3 integration.
//!
//! [`MessageBlob`] is b4's storage metadata for an opaque byte sequence, not a
//! protocol message. b4 never interprets the contents of the blobs it stores.

use base64::Engine;
use serde::{Deserialize, Serialize};

/// Maximum size for inline blob storage (set to 0 to force all blobs to S3 for testing)
pub const MAX_INLINE_MESSAGE_SIZE: usize = 0; // Was: 1024 * 1024 (1MB)

/// A blob stored in the bulletin board.
///
/// b4 is a dumb, board-agnostic blob store (§8): it keeps only the opaque
/// content, the autoincrement `id` (per-board order), and the `version` string
/// for the exact-match boundary check (§10.1). It carries NO protocol metadata
/// (`sender_pk`/`statement_kind`/`batch`/`mix_number` are gone — the slot lives
/// only in datalog `collides()`, §5) and no ops metadata (timestamp/size — a
/// future diagnostics concern, §12).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageBlob {
    pub id: String,
    pub content_type: ContentType,
    pub version: String,
}

/// Content storage type for blobs
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
/// Only size is needed - metadata is extracted from message bytes during confirmation
#[derive(Debug, Serialize, Deserialize)]
pub struct InitiateMessageRequest {
    pub size: usize,
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
    /// Message data (only for inline messages; S3 messages already uploaded)
    pub data: Option<Vec<u8>>,
    /// Protocol version of the message format
    pub version: String,
}

/// Response from confirming a message upload
#[derive(Debug, Serialize, Deserialize)]
pub struct ConfirmMessageResponse {
    pub success: bool,
}

/// Response from getting a single blob
#[derive(Debug, Serialize, Deserialize)]
pub struct GetBlobResponse {
    pub message: MessageBlob,
    pub download_url: Option<String>,
}

/// Response from listing blobs (metadata only)
#[derive(Debug, Serialize, Deserialize)]
pub struct ListBlobsResponse {
    pub messages: Vec<MessageBlob>,
}

/// Blob with pre-signed download URL for S3 content
#[derive(Debug, Serialize, Deserialize)]
pub struct MessageBlobWithUrl {
    #[serde(flatten)]
    pub message: MessageBlob,
    pub download_url: Option<String>,
}

/// Response from getting blobs (includes download URLs for immediate use)
#[derive(Debug, Serialize, Deserialize)]
pub struct GetBlobsResponse {
    pub messages: Vec<MessageBlobWithUrl>,
}

// Multi-board (multiplexing) API types were removed for v0.6 (§8): the board
// client talks to one board per transport (two for a union, §8.2), and b4 has no
// multi-board endpoints. Multiplexing is a possible future optimization, not a
// v0.6 concern.

// Board management types

/// Information about a single board
#[derive(Debug, Serialize, Deserialize)]
pub struct BoardResponse {
    pub name: String,
    pub created_at: i64,
    pub status: String,
}

/// Response listing multiple boards
#[derive(Debug, Serialize, Deserialize)]
pub struct BoardsListResponse {
    pub boards: Vec<BoardResponse>,
}

/// Request to create a new board
#[derive(Debug, Serialize, Deserialize)]
pub struct CreateBoardRequest {
    pub name: String,
}

/// Query parameters for getting blobs
#[derive(Debug, Serialize, Deserialize)]
pub struct GetBlobsQuery {
    pub last_id: Option<i64>,
    pub limit: Option<i64>,
}
