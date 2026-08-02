// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use serde::{Deserialize, Serialize};

/// HTTP-based bulletin board message wrapper.
///
/// This is the HTTP equivalent of GrpcB3Message, designed to work
/// in both native and WASM contexts. It wraps a serialized Message
/// along with metadata needed by the bulletin board.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpB3Message {
    /// Message ID assigned by the bulletin board
    pub id: i64,
    /// Serialized braid::message (from strand_serialize)
    pub message: Vec<u8>,
    /// Schema version for compatibility checking
    pub version: String,
    /// Sender public key (base64-encoded DER SPKI)
    pub sender_pk: String,
    /// Statement kind (e.g., "Configuration", "PublicKey", "PublicKeySigned")
    pub statement_kind: String,
    /// Batch number
    pub batch: i32,
    /// Mix number
    pub mix_number: i32,
}

impl HttpB3Message {
    pub fn new(
        id: i64,
        message: Vec<u8>,
        version: String,
        sender_pk: String,
        statement_kind: String,
        batch: i32,
        mix_number: i32,
    ) -> Self {
        HttpB3Message {
            id,
            message,
            version,
            sender_pk,
            statement_kind,
            batch,
            mix_number,
        }
    }
}

/// HTTP-based board messages container.
///
/// Groups messages by board name, similar to BoardMessages in gRPC.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpBoardMessages {
    pub board: String,
    pub messages: Vec<HttpB3Message>,
}

impl HttpBoardMessages {
    pub fn new(board: String, messages: Vec<HttpB3Message>) -> Self {
        HttpBoardMessages { board, messages }
    }
}
