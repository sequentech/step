use anyhow::Result;

use board_messages::grpc::{GrpcB3Message, KeyedMessages};
use std::io::Write;
use strand::serialization::StrandDeserialize;
use tracing::{info, warn};

use board_messages::braid::message::Message;
use board_messages::grpc::client::B3Client;

const MAX_MESSAGE_SIZE: usize = 1024 * 1024 * 1024;
const GRPC_TIMEOUT: u64 = 5 * 60;

impl BoardMulti for GrpcB3 {
    type Factory = GrpcB3BoardParams;

    async fn get_messages_multi(
        &self,
        requests: &Vec<(String, i64)>,
    ) -> Result<Vec<KeyedMessages>> {
        let response = self.client.get_messages_multi(requests).await?;
        let response = response.into_inner();

        Ok(response.messages)
    }

    async fn insert_messages_multi(&self, requests: Vec<(String, Vec<Message>)>) -> Result<()> {
        let _ = self.client.put_messages_multi(requests).await?;

        Ok(())
    }
}

pub struct GrpcB3Index {
    client: B3Client,
}
impl GrpcB3Index {
    pub fn new(url: &str) -> GrpcB3Index {
        let client = B3Client::new(url, MAX_MESSAGE_SIZE, GRPC_TIMEOUT);

        GrpcB3Index { client }
    }

    pub async fn get_boards(&self) -> Result<Vec<String>> {
        let boards = self.client.get_boards().await?;
        let boards = boards.into_inner();

        Ok(boards.boards)
    }
}

pub struct GrpcB3 {
    client: B3Client,
    step_counter: u64,
}
impl GrpcB3 {
    pub fn new(url: &str) -> GrpcB3 {
        let client = B3Client::new(url, MAX_MESSAGE_SIZE, GRPC_TIMEOUT);

        GrpcB3 {
            client,
            step_counter: 0,
        }
    }

    // Returns all messages whose id > last_id.
    async fn get_remote_messages(&mut self, last_id: i64) -> Result<Vec<GrpcB3Message>> {
        let messages = self.client.get_messages("FOO", last_id).await?;

        let messages = messages.into_inner();

        Ok(messages.messages)
    }
}

pub struct GrpcB3BoardParams {
    pub url: String,
}
impl GrpcB3BoardParams {
    pub fn new(url: &str) -> GrpcB3BoardParams {
        GrpcB3BoardParams {
            url: url.to_string(),
        }
    }
}

impl BoardFactoryMulti<GrpcB3> for GrpcB3BoardParams {
    fn get_board(&self) -> GrpcB3 {
        GrpcB3::new(&self.url)
    }
}

pub trait BoardMulti: Sized {
    type Factory: BoardFactoryMulti<Self>;

    fn get_messages_multi(
        &self,
        requests: &Vec<(String, i64)>,
    ) -> impl std::future::Future<Output = Result<Vec<KeyedMessages>>> + Send;
    // ) -> impl std::future::Future<Output = Result<Vec<(String, Vec<GrpcB3Message>)>>> + Send;

    fn insert_messages_multi(
        &self,
        requests: Vec<(String, Vec<Message>)>,
    ) -> impl std::future::Future<Output = Result<()>> + Send;
}

pub trait BoardFactoryMulti<B: BoardMulti>: Sized {
    fn get_board(&self) -> B;
}

#[cfg(test)]
pub(crate) mod tests {

    use board_messages::grpc::B3Client;
    use board_messages::grpc::GetMessagesRequest;
    use serial_test::serial;

    #[tokio::test]
    #[ignore]
    #[serial]
    async fn test_grpc_client() {
        let mut client = B3Client::connect("http://[::1]:50051").await.unwrap();

        let request = tonic::Request::new(GetMessagesRequest {
            board: "default".to_string(),
            last_id: -1,
        });

        let response = client.get_messages(request).await.unwrap();

        println!("RESPONSE={:?}", response.into_inner().messages);
    }
}
