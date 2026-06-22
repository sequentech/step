// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use serde::{Deserialize, Serialize};
use cryptography::context::Context;
use cryptography::utils::serialization::variable::VSerializable;
use crate::messages::message::Message;

/// HTTP-based bulletin board message wrapper.
///
/// Designed to work in both native and WASM contexts. It wraps a serialized Message
/// along with the bulletin board ID and schema version.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpB4Message {
    /// Message ID assigned by the bulletin board
    pub id: i64,
    /// Serialized braid::message
    pub message: Vec<u8>,
    /// Schema version for compatibility checking
    pub version: String,
}

impl HttpB4Message {
    pub fn new(id: i64, message: Vec<u8>, version: String) -> Self {
        HttpB4Message {
            id,
            message,
            version,
        }
    }

    /// Create HttpB4Message from a protocol message for posting to bulletin board
    /// 
    /// The `id` field is set to 0 since it will be assigned by the bulletin board
    /// upon storage. The version is set to the current schema version.
    /// 
    /// This conversion marks the boundary between the protocol layer (Message<C>)
    /// and the wire/transport layer (HttpB4Message).
    pub fn from_protocol_message<C: Context>(message: Message<C>) -> Self {
        let message_bytes = message.ser();
        HttpB4Message {
            id: 0,  // Will be assigned by bulletin board
            message: message_bytes,
            version: crate::get_schema_version(),
        }
    }
}

/// HTTP-based board messages container.
///
/// Groups messages by board name.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpBoardMessages {
    pub board: String,
    pub messages: Vec<HttpB4Message>,
}

impl HttpBoardMessages {
    pub fn new(board: String, messages: Vec<HttpB4Message>) -> Self {
        HttpBoardMessages { board, messages }
    }
}
