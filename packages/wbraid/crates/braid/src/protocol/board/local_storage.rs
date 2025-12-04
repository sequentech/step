// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Storage backend abstraction for LocalBoard
//!
//! This module defines the LocalBoardStorage trait that separates
//! persistence mechanisms (SQLite, IndexedDB, in-memory) from the
//! LocalBoard business logic.

use anyhow::Result;
use b4::HttpB3Message;
use b4::messages::message::Message;

/// Storage backend abstraction for LocalBoard persistence
///
/// This trait defines the interface for message persistence, separating
/// the storage mechanism (SQLite, IndexedDB, in-memory) from the LocalBoard
/// business logic.
///
/// # Security Model
///
/// Implementations MUST ensure:
/// - **Append-only**: Messages get auto-incrementing local IDs that cannot be deleted
/// - **Tamper-resistant**: Uniqueness constraints prevent duplicate messages
/// - **Locally-controlled ordering**: Local IDs determine processing order, not external IDs
///
/// The `store_messages()` method assigns locally-controlled IDs (e.g., SQLite AUTOINCREMENT)
/// that establish immutable message ordering. The `retrieve_messages()` method returns messages
/// in this locally-determined order, preventing the bulletin board from manipulating history.
pub trait LocalBoardStorage: Send + Sync {
    /// Store messages and assign locally-controlled IDs
    ///
    /// # Security
    ///
    /// - Each message MUST be assigned a new auto-incrementing local ID
    /// - Duplicate detection via UNIQUE constraint on (sender_pk, kind, batch, mix_number)
    /// - If `ignore_existing` is true, silently skip duplicates (used for periodic full refresh)
    ///
    /// # Parameters
    ///
    /// - `messages`: Messages from bulletin board with external IDs
    /// - `ignore_existing`: If true, silently ignore duplicate inserts (for full refresh)
    fn store_messages(&self, messages: &[HttpB3Message], ignore_existing: bool) -> Result<()>;

    /// Retrieve messages with local_id > last_local_board_id
    ///
    /// # Security
    ///
    /// MUST return messages ordered by locally-controlled ID (e.g., `ORDER BY id ASC`).
    /// This ensures the bulletin board cannot manipulate message ordering.
    ///
    /// # Returns
    ///
    /// Vector of (Message, local_id) pairs in ascending local_id order
    fn retrieve_messages(&self, last_local_board_id: i64) -> Result<Vec<(Message, i64)>>;

    /// Get the maximum external_id stored
    ///
    /// # Optimization Only
    ///
    /// This is used to request `messages WHERE external_id > X` from the bulletin board,
    /// avoiding redundant fetches. Has NO security implications - the bulletin board
    /// controls external IDs, but our local IDs control processing order.
    ///
    /// # Returns
    ///
    /// Maximum external_id, or -1 if store is empty
    fn get_last_external_id(&self) -> Result<i64>;
}
