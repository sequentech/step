pub mod proto {
    tonic::include_proto!("b3"); // The string specified here must match the proto package name
}

pub use proto::b3_client::B3Client;
pub use proto::b3_server::B3Server;
pub use proto::GetBoardsReply;
pub use proto::GetBoardsRequest;
pub use proto::GetMessagesMultiReply;
pub use proto::GetMessagesMultiRequest;
pub use proto::GetMessagesReply;
pub use proto::GetMessagesRequest;
pub use proto::GrpcB3Message;
pub use proto::KeyedMessages;
pub use proto::PutMessagesMultiReply;
pub use proto::PutMessagesMultiRequest;
pub use proto::PutMessagesReply;
pub use proto::PutMessagesRequest;

pub mod client;
pub mod pgsql;
pub mod server;

pub(crate) const MAX_MESSAGE_SIZE: usize = 2 * 1024 * 1024 * 1024;

pub(crate) fn validate_board_name(board: &str) -> anyhow::Result<()> {
    if board.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        Ok(())
    } else {
        Err(anyhow::anyhow!("Invalid board name {}", board))
    }
}
