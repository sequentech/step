// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::Result;

use b4::{HttpB3Message, HttpBoardMessages};

use b4::client::grpc::B3Client;
use b4::messages::message::Message;
use strand::serialization::StrandDeserialize;

use super::BoardFactory;

/// A large upper bound on grpc message size.
///
/// In practice, message size is constrained by the
/// smaller value b4::grpc::MAX_MESSAGE_SIZE
/// that determines chunking behaviour.
const MAX_MESSAGE_SIZE: usize = 10 * 1024 * 1024 * 1024;

/// A large upper bound on grpc timeout.
const GRPC_TIMEOUT: u64 = 5 * 60;

/// A grpc client of a braid bulletin board (B3).
///
/// Used to retrieve and post protocol messages from
/// the bulletin board. This client implements both
/// the standard client functions (super::Board) as
/// well as their multiplexing versions (super::BoardMulti)
pub struct HttpB3 {
    client: B3Client,
}
impl HttpB3 {
    /// Constructs a HttpB3 that will query the target url.
    pub fn new(url: &str) -> HttpB3 {
        let client = B3Client::new(url, MAX_MESSAGE_SIZE, GRPC_TIMEOUT);

        HttpB3 { client }
    }
}

impl super::BoardMulti for HttpB3 {
    type Factory = HttpB3BoardParams;

    async fn get_messages_multi(
        &self,
        requests: &Vec<(String, i64)>,
    ) -> Result<(Vec<HttpBoardMessages>, bool)> {
        let response = self.client.get_messages_multi(requests).await?;
        let response = response.into_inner();
        
        // Convert gRPC BoardMessages to HttpBoardMessages
        let http_messages = response.messages.into_iter()
            .map(|bm| HttpBoardMessages {
                board: bm.board,
                messages: bm.messages.into_iter()
                    .filter_map(|gm| {
                        // Deserialize to extract metadata
                        let msg = Message::strand_deserialize(&gm.message).ok()?;
                        Some(HttpB3Message::new(
                            gm.id,
                            gm.message,
                            gm.version,
                            msg.sender.pk.to_der_b64_string().ok()?,
                            msg.statement.get_kind().to_string(),
                            msg.statement.get_batch_number().try_into().ok()?,
                            msg.statement.get_mix_number().try_into().ok()?,
                        ))
                    })
                    .collect(),
            })
            .collect();

        Ok((http_messages, response.truncated))
    }

    async fn insert_messages_multi(&self, requests: Vec<(String, Vec<Message>)>) -> Result<()> {
        let _ = self.client.put_messages_multi(requests).await?;

        Ok(())
    }
}

impl super::Board for HttpB3 {
    type Factory = HttpB3BoardParams;
    async fn get_messages(&mut self, board: &str, last_id: i64) -> Result<Vec<HttpB3Message>> {
        let messages = self.client.get_messages(board, last_id).await?;

        let messages = messages.into_inner();
        
        // Convert gRPC messages to HTTP messages
        let http_messages = messages.messages.into_iter()
            .filter_map(|gm| {
                // Deserialize to extract metadata
                let msg = Message::strand_deserialize(&gm.message).ok()?;
                Some(HttpB3Message::new(
                    gm.id,
                    gm.message,
                    gm.version,
                    msg.sender.pk.to_der_b64_string().ok()?,
                    msg.statement.get_kind().to_string(),
                    msg.statement.get_batch_number().try_into().ok()?,
                    msg.statement.get_mix_number().try_into().ok()?,
                ))
            })
            .collect();

        Ok(http_messages)
    }
    async fn insert_messages(&mut self, board: &str, messages: Vec<Message>) -> Result<()> {
        if messages.len() > 0 {
            self.client.put_messages(board, &messages).await?;
        }

        Ok(())
    }
}

/// A grpc client of a braid bulletin board (B3) index.
///
/// The bulletin board index lists all active and
/// archived boards for which the protocol must be
/// or has been run.
pub struct HttpB3Index {
    client: B3Client,
}
impl HttpB3Index {
    /// Constructs a HttpB3Index that will query the target url.
    pub fn new(url: &str) -> HttpB3Index {
        let client = B3Client::new(url, MAX_MESSAGE_SIZE, GRPC_TIMEOUT);

        HttpB3Index { client }
    }

    /// Returns the list of active boards from the index.
    pub async fn get_boards(&self) -> Result<Vec<String>> {
        let boards = self.client.get_boards().await?;
        let boards = boards.into_inner();

        let ret: Vec<String> = boards
            .boards
            .into_iter()
            .filter(|b| Self::is_board_name_valid(b))
            .collect();

        Ok(ret)
    }

    /// Whether the board name is valid, as defined in
    /// b3.
    fn is_board_name_valid(name: &str) -> bool {
        if b4::grpc::validate_board_name(name).is_ok() {
            true
        } else {
            tracing::warn!("Received an invalid board name: {}", name);
            false
        }
    }
}

/// The parameters necessary to construct a HttpB3 client.
///
/// This object serves as a HttpB3 client factory,
/// implementing BoardFactory and BoardFactoryMulti.
pub struct HttpB3BoardParams {
    pub url: String,
}
impl HttpB3BoardParams {
    pub fn new(url: &str) -> HttpB3BoardParams {
        HttpB3BoardParams {
            url: url.to_string(),
        }
    }
}

impl BoardFactory<HttpB3> for HttpB3BoardParams {
    fn get_board(&self) -> HttpB3 {
        HttpB3::new(&self.url)
    }
}
impl super::BoardFactoryMulti<HttpB3> for HttpB3BoardParams {
    fn get_board(&self) -> HttpB3 {
        HttpB3::new(&self.url)
    }
}

#[cfg(test)]
pub(crate) mod tests {

    use b4::grpc::B3Client;
    use b4::grpc::GetMessagesRequest;
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
