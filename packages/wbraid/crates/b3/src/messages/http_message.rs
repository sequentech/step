// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
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
}

impl HttpB3Message {
    pub fn new(id: i64, message: Vec<u8>, version: String) -> Self {
        HttpB3Message {
            id,
            message,
            version,
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
