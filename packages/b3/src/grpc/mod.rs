// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
pub mod proto {
    tonic::include_proto!("b3"); // The string specified here must match the proto package name
}

pub use proto::b3_client::B3Client;
pub use proto::b3_server::B3Server;
pub use proto::BoardMessages;
pub use proto::GetBoardsReply;
pub use proto::GetBoardsRequest;
pub use proto::GetMessagesMultiReply;
pub use proto::GetMessagesMultiRequest;
pub use proto::GetMessagesReply;
pub use proto::GetMessagesRequest;
pub use proto::GrpcB3Message;
pub use proto::PutMessagesMultiReply;
pub use proto::PutMessagesMultiRequest;
pub use proto::PutMessagesReply;
pub use proto::PutMessagesRequest;

#[cfg(feature = "server")]
pub mod server;

/// The maximum grpc message used for chunking.
///
/// Values greater than 2MB have caused external (tonic) grpc
/// errors during testing.
pub const MESSAGE_CHUNK_SIZE: usize = 2 * 1024 * 1024 * 1024;

/// The maximum grpc message set for transports.
///
/// Values greater than 2MB have caused external (tonic) grpc
/// errors during testing.
pub const MAX_MESSAGE_SIZE: usize = 10 * 1024 * 1024 * 1024;

/// Returns Ok if the board name is valid, otherwise returns and error.
pub fn validate_board_name(board: &str) -> anyhow::Result<()> {
    if board.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        Ok(())
    } else {
        Err(anyhow::anyhow!("Invalid board name {}", board))
    }
}
