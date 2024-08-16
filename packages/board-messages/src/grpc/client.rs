use crate::braid::message::Message;

use strand::serialization::StrandSerialize;
use tonic::transport::Endpoint;

use super::GetMessagesReply;
use super::{B3Client as B3ClientInner, GetBoardsReply, GetBoardsRequest, GetMessagesRequest, GrpcB3Message, PutMessagesReply, PutMessagesRequest};
use anyhow::Result;
use tonic::Request;
use tonic::{transport::Channel, Response};

pub struct B3Client {
    url: String
}

impl B3Client {
    pub fn new(url: &str) -> B3Client {
        B3Client {
            url: url.to_string(),
        }
    }

    pub async fn get_messages(&self, board: &str, last_id: i64) -> Result<Response<GetMessagesReply>> {
        let request = Self::get_messages_request(board, last_id);
    
        let mut client = self.get_grpc_client().await?;
        let response = client.get_messages(request).await?;

        Ok(response)
    }

    pub async fn put_messages(&self, board: &str, messages: &[Message]) -> Result<Response<PutMessagesReply>> {
        let request = Self::put_messages_request(board, messages)?;
    
        let mut client = self.get_grpc_client().await?;
        let response = client.put_messages(request).await?;

        Ok(response)
    }

    pub async fn get_boards(&self) -> Result<Response<GetBoardsReply>> {
        let request = Self::get_boards_request();

        let mut client = self.get_grpc_client().await?;
        let response = client.get_boards(request).await?;

        Ok(response)
    }

    pub(crate) fn put_messages_request(board: &str, messages: &[Message]) -> Result<Request<PutMessagesRequest>> {
        let messages: Result<Vec<GrpcB3Message>> = messages.into_iter().map(|m| {
            let message = GrpcB3Message {
                id: 0,
                message: m.strand_serialize()?,
                version: crate::get_schema_version(),
            };

            Ok(message)
        }).collect();

        let request = Request::new(PutMessagesRequest {
            board: board.to_string(),
            messages: messages?,
        });

        Ok(request)
    }

    pub(crate) fn get_messages_request(board: &str, last_id: i64) -> Request<GetMessagesRequest> {
        Request::new(GetMessagesRequest {
            board: board.to_string(),
            last_id,
        })
    }

    pub(crate) fn get_boards_request() -> Request<GetBoardsRequest> {
        Request::new(GetBoardsRequest {
        })
    }

    pub(crate) async fn get_grpc_client(&self) -> Result<B3ClientInner<Channel>> {
        let endpoint = Endpoint::from_shared(self.url.clone())?;
        let client = B3ClientInner::connect(endpoint).await?;

        Ok(client)
    }
}