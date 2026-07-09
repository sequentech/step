// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

/// Test the protocol using an HTTP+S3 board.
pub mod protocol_test_http;
/// Test the protocol using an in memory board.
pub mod protocol_test_memory;
// Legacy `dbg`, `vector_board`, `vector_session` are retired from the build for
// M2; their files remain on disk for reference.
