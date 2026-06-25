// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! No-op storage backend for LocalBoard
//!
//! This is a pass-through implementation that provides no persistence.
//! Messages are temporarily held between store_messages() and retrieve_messages()
//! calls within a single step, then discarded.
//!
//! # Security Warning
//!
//! This storage provides NO security guarantees:
//! - No tamper resistance (no AUTOINCREMENT local IDs)
//! - No append-only guarantee
//! - Uses external bulletin board IDs directly (can be manipulated)
//!
//! # Use Cases
//!
//! - **Testing only**: When persistence and security are not needed
//! - **WASM (temporary)**: Until async IndexedDB implementation is complete
//! - **Verifier**: When running verification on existing message sets

use anyhow::Result;
use std::sync::Mutex;
use strand::serialization::StrandDeserialize;

use crate::protocol::board::local_storage::LocalBoardStorage;
use b4::messages::message::Message;
use b4::HttpB3Message;

/// No-op storage backend (no persistence, no security)
///
/// This implementation satisfies the LocalBoardStorage trait interface
/// but provides no actual persistence. Messages are held transiently
/// between store and retrieve calls, using external bulletin board IDs.
///
/// WARNING: Provides no security guarantees - for testing only.
pub struct NoOpStorage {
    /// Transient buffer holding messages between store_messages() and retrieve_messages()
    /// within a single protocol step. Cleared after retrieval.
    /// Uses Mutex for thread-safety (required for parallel action execution).
    transient: Mutex<Vec<HttpB3Message>>,
}

impl NoOpStorage {
    pub fn new() -> Self {
        NoOpStorage {
            transient: Mutex::new(Vec::new()),
        }
    }
}

impl LocalBoardStorage for NoOpStorage {
    fn store_messages(&self, messages: &[HttpB3Message], _ignore_existing: bool) -> Result<()> {
        // Store messages transiently for this step only
        let mut transient = self.transient.lock().unwrap();
        *transient = messages.to_vec();
        Ok(())
    }

    fn retrieve_messages(&self, _last_local_board_id: i64) -> Result<Vec<(Message, i64)>> {
        // Parse messages from transient buffer, using external IDs
        let mut transient = self.transient.lock().unwrap();
        let result: Result<Vec<(Message, i64)>> = transient
            .iter()
            .map(|m| {
                let message = Message::strand_deserialize(&m.message)?;
                Ok((message, m.id)) // Use external bulletin board ID (no security)
            })
            .collect();

        // Clear transient buffer after retrieval
        transient.clear();

        result
    }

    fn get_last_external_id(&self) -> Result<i64> {
        // No-op storage doesn't track IDs - always request all messages
        Ok(-1)
    }

    fn get_storage_info(&self) -> Result<crate::protocol::board::StorageInfo> {
        use crate::protocol::board::StorageInfo;
        Ok(StorageInfo {
            backend_type: "NoOpStorage (transient)".to_string(),
            total_messages: 0,
            max_internal_id: -1,
            max_external_id: -1,
            extra_info: Some("No persistence - messages cleared after each step".to_string()),
        })
    }
}
