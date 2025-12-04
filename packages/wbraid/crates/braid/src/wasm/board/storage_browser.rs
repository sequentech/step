// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Browser-based storage implementation using localStorage
//!
//! WARNING: This is a simplified implementation that stores messages transiently.
//! A production implementation should use IndexedDB for proper persistence and
//! security guarantees (append-only, auto-incrementing local IDs).

use std::sync::Mutex;
use strand::serialization::StrandDeserialize;

use b4::messages::message::Message;
use b4::HttpB3Message;
use crate::protocol::board::LocalBoardStorage;

/// Browser storage implementation (currently transient, TODO: use IndexedDB)
/// 
/// This implementation satisfies the LocalBoardStorage trait but currently
/// provides no actual persistence. A future implementation should use IndexedDB
/// to provide proper persistent storage across browser sessions with security
/// guarantees (append-only via auto-incrementing local IDs).
pub struct BrowserStorage {
    /// Transient buffer holding messages between store_messages() and retrieve_messages()
    /// Uses Mutex for thread-safety.
    transient: Mutex<Vec<HttpB3Message>>,
}

impl BrowserStorage {
    pub fn new() -> Self {
        BrowserStorage {
            transient: Mutex::new(Vec::new()),
        }
    }
}

impl LocalBoardStorage for BrowserStorage {
    fn store_messages(&self, messages: &[HttpB3Message], _ignore_existing: bool) -> anyhow::Result<()> {
        // Store messages transiently for this step only
        // TODO: Replace with IndexedDB implementation for real persistence
        let mut transient = self.transient.lock().unwrap();
        *transient = messages.to_vec();
        Ok(())
    }

    fn retrieve_messages(&self, _last_local_board_id: i64) -> anyhow::Result<Vec<(Message, i64)>> {
        // Parse messages from transient buffer, using external IDs
        // TODO: Replace with IndexedDB implementation with local auto-incrementing IDs
        let mut transient = self.transient.lock().unwrap();
        let result: anyhow::Result<Vec<(Message, i64)>> = transient
            .iter()
            .map(|m| {
                let message = Message::strand_deserialize(&m.message)?;
                Ok((message, m.id)) // Use external bulletin board ID (temporary)
            })
            .collect();
        
        // Clear transient buffer after retrieval
        transient.clear();
        
        result
    }

    fn get_last_external_id(&self) -> anyhow::Result<i64> {
        // Transient storage doesn't track IDs - always request all messages
        // TODO: Implement proper tracking with IndexedDB
        Ok(-1)
    }
}

impl Default for BrowserStorage {
    fn default() -> Self {
        Self::new()
    }
}
