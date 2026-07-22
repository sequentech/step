// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

/// Test the protocol using an HTTP+S3 board.
pub mod protocol_test_http;
/// Test the DKG+tally board union (§8.2) over an HTTP+S3 board with SQLite
/// persistence.
pub mod protocol_test_http_union;
/// Test the protocol using an in memory board.
pub mod protocol_test_memory;
/// Test the DKG+tally board union (§8.2) using in-memory boards.
pub mod protocol_test_memory_union;
