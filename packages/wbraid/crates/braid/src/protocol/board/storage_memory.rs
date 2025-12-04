// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! In-memory storage backend for LocalBoard
//!
//! This implementation provides no persistence - all data is lost when
//! the program terminates. Used for:
//! - Testing
//! - WASM (until IndexedDB async implementation is ready)
//! - Scenarios where persistence is not required

use anyhow::Result;
use b4::messages::message::Message;
use b4::HttpB3Message;
use crate::protocol::board::local_storage::LocalBoardStorage;

/// In-memory storage backend (no persistence)
///
/// This is a stub implementation that satisfies the LocalBoardStorage trait
/// but provides no actual persistence. Messages are never stored, and
/// retrieval always returns an empty vector.
///
/// # Use Cases
///
/// - **Testing**: When persistence is not needed for tests
/// - **WASM (current)**: Until async IndexedDB implementation is complete
/// - **Verifier**: When running verification on existing message sets
pub struct InMemoryStorage;

impl InMemoryStorage {
    pub fn new() -> Self {
        InMemoryStorage
    }
}

impl LocalBoardStorage for InMemoryStorage {
    fn store_messages(&self, _messages: &[HttpB3Message], _ignore_existing: bool) -> Result<()> {
        // No-op: in-memory storage doesn't persist
        Ok(())
    }

    fn retrieve_messages(&self, _last_local_board_id: i64) -> Result<Vec<(Message, i64)>> {
        // No persistence, so no messages to retrieve
        Ok(vec![])
    }

    fn get_last_external_id(&self) -> Result<i64> {
        // No persistence, return sentinel value
        Ok(-1)
    }
}
