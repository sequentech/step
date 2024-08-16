pub mod proto {
    tonic::include_proto!("b3"); // The string specified here must match the proto package name
}

pub use proto::b3_client::B3Client;
pub use proto::b3_server::B3Server;
pub use proto::GetMessagesRequest;
pub use proto::GetMessagesReply;
pub use proto::GetBoardsRequest;
pub use proto::GetBoardsReply;
pub use proto::GrpcB3Message;
pub use proto::PutMessagesRequest;
pub use proto::PutMessagesReply;

pub mod client;
pub mod server;
pub mod pgsql;

pub(crate) fn validate_board_name(board: &str) -> anyhow::Result<()> {
    // FIXME
    Ok(())
}