// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// Storage trait (persistence abstraction)
pub mod local_storage;

// Universal LocalBoard implementation and data structures
pub mod local_board;

// Re-export LocalBoard and its data structures
pub use local_board::{ArtifactEntryIdentifier, BoardEntry, LocalBoard, StatementEntryIdentifier};

// Re-export storage trait
pub use local_storage::LocalBoardStorage;

use anyhow::Result;
use b4::messages::message::Message;
use b4::HttpB3Message;

/// Defines the interface with a bulletin board.
///
/// The trustee interactions with the bulletin board are
/// limited to two cases.
///
/// 1) retrieving messages greater than some id (as defined by the bulletin board).
/// 2) Posting new messages.
pub trait Board: Sized {
    type Factory: BoardFactory<Self>;

    /// Return messages with an id greater than the supplied last_id value from
    /// the given board of the bulletin board.
    ///
    /// The bulletin board assigns ids to messages as they are published by
    /// trustees. This operation allows retrieving messages which the trustee
    /// has not yet obtained. Although they usually match, the bulletin board
    /// ids do not determine the message history; this history is defined
    /// locally by each trustee according to the order in which those messages
    /// were received.
    fn get_messages(
        &mut self,
        board: &str,
        last_id: i64,
    ) -> impl std::future::Future<Output = Result<Vec<HttpB3Message>>> + Send;

    /// Posts a messages to the given board of the bulletin board.
    fn insert_messages(
        &mut self,
        board: &str,
        messages: Vec<Message>,
    ) -> impl std::future::Future<Output = Result<()>> + Send;
}

/// Allows abstracting over a board client implementation
///
/// FIXME: probably overengineered.
pub trait BoardFactory<B: Board>: Sized {
    fn get_board(&self) -> B;
}

/// Defines the interface with the bulletin board, multiplexed version.
///
/// The trustee interactions with the bulletin board are
/// limited to two cases.
///
/// 1) retrieving messages greater than some id (as defined by the bulletin board).
/// 2) Posting new messages.
///
/// This version allows receiving and posting messages in batches that span
/// more than one board.
pub trait BoardMulti: Sized {
    type Factory: BoardFactoryMulti<Self>;

    /// Returns a list of HttpBoardMessages for the given requests.
    ///
    /// HttpBoardMessages are a list of messages for one board.
    fn get_messages_multi(
        &self,
        requests: &Vec<(String, i64)>,
    ) -> impl std::future::Future<Output = Result<(Vec<b4::HttpBoardMessages>, bool)>> + Send;

    fn insert_messages_multi(
        &self,
        requests: Vec<(String, Vec<Message>)>,
    ) -> impl std::future::Future<Output = Result<()>> + Send;
}

/// Allows abstracting over a board client implementation
///
/// FIXME: probably overengineered.
pub trait BoardFactoryMulti<B: BoardMulti>: Sized {
    fn get_board(&self) -> B;
}
