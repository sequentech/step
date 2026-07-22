// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Shared SQL schema for SQLite storage backends
//!
//! This module defines the SQL schema used by both native and WASM
//! SqliteStorage implementations, ensuring consistency across platforms.
//!
//! # Security Model
//!
//! The schema enforces critical security properties:
//!
//! - **Locally-controlled ordering**: AUTOINCREMENT PRIMARY KEY provides
//!   immutable, locally-determined insertion order
//! - **Duplicate prevention**: UNIQUE constraints prevent message replay
//! - **Append-only**: No DELETE or UPDATE operations are used
//!
//! # Schema Version
//!
//! The current schema version is tracked in SCHEMA_VERSION. When the schema
//! changes, increment this version to enable migration logic.

/// Current schema version
pub const SCHEMA_VERSION: i32 = 1;

/// SQL statement to create the MESSAGES table
///
/// # Table Structure
///
/// - `id`: AUTOINCREMENT PRIMARY KEY - locally-controlled ordering (SECURITY CRITICAL)
/// - `external_id`: Bulletin board's ID (optimization only, not security-critical)
/// - `message`: Serialized message bytes (BLOB)
/// - `sender_pk`: Sender's public key (TEXT, DER base64)
/// - `statement_kind`: Type of statement (Configuration, DkgPublicKey, etc.)
/// - `batch`: Batch number (for multi-batch protocols)
/// - `mix_number`: Mix number within batch
///
/// # Constraints
///
/// - PRIMARY KEY on `id` ensures unique, ordered access
/// - UNIQUE on `external_id` prevents bulletin board from sending duplicates
/// - UNIQUE on (sender_pk, statement_kind, batch, mix_number) prevents replays
pub const CREATE_TABLE_SQL: &str = "
CREATE TABLE IF NOT EXISTS MESSAGES(
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    external_id INT8 NOT NULL UNIQUE,
    message BLOB NOT NULL,
    sender_pk TEXT NOT NULL,
    statement_kind TEXT NOT NULL,
    batch INT4 NOT NULL,
    mix_number INT4 NOT NULL,
    UNIQUE(sender_pk, statement_kind, batch, mix_number)
)";

/// SQL statement to insert a message
///
/// Parameters (in order):
/// 1. external_id (i64)
/// 2. message (BLOB)
/// 3. sender_pk (TEXT)
/// 4. statement_kind (TEXT)
/// 5. batch (i32)
/// 6. mix_number (i32)
pub const INSERT_MESSAGE_SQL: &str = "
INSERT INTO MESSAGES (external_id, message, sender_pk, statement_kind, batch, mix_number)
VALUES (?1, ?2, ?3, ?4, ?5, ?6)";

/// SQL statement to retrieve messages after a given local ID
///
/// This is the security-critical query - it uses the local AUTOINCREMENT id
/// to ensure messages are processed in the order they were first stored,
/// regardless of bulletin board behavior.
///
/// Parameters:
/// 1. last_local_id (i64)
///
/// Returns: id, external_id, message (ordered by id ASC)
pub const RETRIEVE_MESSAGES_SQL: &str = "
SELECT id, external_id, message
FROM MESSAGES
WHERE id > ?1
ORDER BY id ASC";

/// SQL statement to get the maximum external_id
///
/// This is used for optimization only - to avoid re-fetching messages
/// from the bulletin board. The external_id has no security implications.
///
/// Returns: max(external_id) or NULL if no messages
pub const GET_LAST_EXTERNAL_ID_SQL: &str = "
SELECT MAX(external_id) FROM MESSAGES";

/// SQL statement to count total messages
///
/// Returns: count(*)
pub const COUNT_MESSAGES_SQL: &str = "
SELECT COUNT(*) FROM MESSAGES";

/// SQL statement to get storage statistics
///
/// Returns: total messages, max internal id, max external id
pub const GET_STORAGE_INFO_SQL: &str = "
SELECT 
    COUNT(*) as total_messages,
    IFNULL(MAX(id), -1) as max_internal_id,
    IFNULL(MAX(external_id), -1) as max_external_id
FROM MESSAGES";
