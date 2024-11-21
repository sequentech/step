// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

/// Interactive simulation of the protocol.
pub mod dbg;
/// Test the protocol using a grpc board.
pub mod protocol_test_grpc;
/// Test the protocol using an in memory board.
pub mod protocol_test_memory;
/// An in-memory board.
pub mod vector_board;
/// An in-memory session (for one trustee).
pub mod vector_session;
