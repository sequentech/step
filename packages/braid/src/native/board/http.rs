// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use tracing::info;

use b4::messages::message::Message;
use b4::HttpB3Message;
use strand::serialization::StrandSerialize;

use crate::protocol::board::{Board, BoardFactory, BoardFactoryMulti, BoardMulti};

#[derive(Debug, Serialize)]
struct InitiateMessageRequest {
    size: usize,
    sender_pk: String,
    statement_kind: String,
    batch: i32,
    mix_number: i32,
}

#[derive(Debug, Deserialize)]
struct InitiateMessageResponse {
    message_id: String,
    upload_url: Option<String>,
    should_upload: bool,
}

#[derive(Debug, Serialize)]
struct ConfirmMessageRequest {
    data: Option<Vec<u8>>,
    sender_pk: String,
    statement_kind: String,
    batch: i32,
    mix_number: i32,
}

#[derive(Debug, Deserialize)]
struct ConfirmMessageResponse {
    success: bool,
}

#[derive(Debug, Deserialize)]
struct GetMessagesResponse {
    messages: Vec<MessageWithUrl>,
}

#[derive(Debug, Deserialize)]
struct MessageWithUrl {
    id: String,
    #[allow(dead_code)]
    timestamp: i64,
    #[allow(dead_code)]
    size: usize,
    content_type: ContentTypeDto,
    sender_pk: String,
    statement_kind: String,
    batch: i32,
    mix_number: i32,
    download_url: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum ContentTypeDto {
    Inline {
        message: String,
    }, // base64 encoded
    S3 {
        #[allow(dead_code)]
        key: String,
    },
}

/// HTTP client for bulletin board using Service API
pub struct HttpB3 {
    client: reqwest::Client,
    base_url: String,
    s3_client: aws_sdk_s3::Client,
    bucket_name: String,
    access_token: Arc<RwLock<String>>,
}

impl HttpB3 {
    pub async fn new(
        base_url: &str,
        s3_client: aws_sdk_s3::Client,
        bucket_name: &str,
        access_token: String,
    ) -> HttpB3 {
        HttpB3 {
            client: reqwest::Client::new(),
            base_url: base_url.to_string(),
            s3_client,
            bucket_name: bucket_name.to_string(),
            access_token: Arc::new(RwLock::new(access_token)),
        }
    }

    /// Adds the Authorization header for B4 authentication
    fn add_auth_header(&self, request: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        let access_token = self
            .access_token
            .read()
            .expect("access_token lock poisoned")
            .clone();
        request.header("Authorization", format!("Bearer {}", access_token))
    }

    /// Helper to post a single message to a specific board
    async fn post_message_to_board(&self, board: &str, message: &Message) -> Result<()> {
        // Extract metadata from message
        let sender_pk = message.sender.pk.to_der_b64_string()?;
        let statement_kind = message.statement.get_kind().to_string();
        let batch: i32 = message.statement.get_batch_number().try_into()?;
        let mix_number: i32 = message.statement.get_mix_number().try_into()?;

        // Serialize the message
        let message_bytes = message.strand_serialize()?;
        let size = message_bytes.len();

        // Phase 1: Initiate message
        let initiate_url = format!("{}/boards/{}/messages/initiate", self.base_url, board);
        let initiate_req = InitiateMessageRequest {
            size,
            sender_pk: sender_pk.clone(),
            statement_kind: statement_kind.clone(),
            batch,
            mix_number,
        };

        let initiate_response = self
            .add_auth_header(self.client.post(&initiate_url))
            .json(&initiate_req)
            .send()
            .await?;

        if !initiate_response.status().is_success() {
            anyhow::bail!(
                "Failed to initiate message: HTTP {}",
                initiate_response.status()
            );
        }

        let init_resp: InitiateMessageResponse = initiate_response.json().await?;

        // Phase 2: Upload data (to S3 if large, inline if small)
        if init_resp.should_upload {
            // Large message - upload to S3
            if let Some(upload_url) = &init_resp.upload_url {
                let s3_response = self
                    .client
                    .put(upload_url)
                    .body(message_bytes.clone())
                    .send()
                    .await?;

                if !s3_response.status().is_success() {
                    anyhow::bail!("Failed to upload to S3: HTTP {}", s3_response.status());
                }

                // Phase 3: Confirm (no data for S3 messages)
                let confirm_url = format!(
                    "{}/boards/{}/messages/{}/confirm",
                    self.base_url, board, init_resp.message_id
                );
                let confirm_req = ConfirmMessageRequest {
                    data: None,
                    sender_pk: sender_pk.clone(),
                    statement_kind: statement_kind.clone(),
                    batch,
                    mix_number,
                };

                let confirm_response = self
                    .add_auth_header(self.client.post(&confirm_url))
                    .json(&confirm_req)
                    .send()
                    .await?;

                if !confirm_response.status().is_success() {
                    anyhow::bail!(
                        "Failed to confirm S3 message: HTTP {}",
                        confirm_response.status()
                    );
                }
            } else {
                anyhow::bail!("Server indicated upload needed but provided no URL");
            }
        } else {
            // Small message - send inline
            let confirm_url = format!(
                "{}/boards/{}/messages/{}/confirm",
                self.base_url, board, init_resp.message_id
            );
            let confirm_req = ConfirmMessageRequest {
                data: Some(message_bytes),
                sender_pk,
                statement_kind,
                batch,
                mix_number,
            };

            let confirm_response = self
                .add_auth_header(self.client.post(&confirm_url))
                .json(&confirm_req)
                .send()
                .await?;

            if !confirm_response.status().is_success() {
                anyhow::bail!(
                    "Failed to confirm inline message: HTTP {}",
                    confirm_response.status()
                );
            }
        }

        info!(
            "Inserted message to board {}, ID: {}, size: {} bytes",
            board, init_resp.message_id, size
        );

        Ok(())
    }
}

impl Board for HttpB3 {
    type Factory = HttpB3BoardParams;

    async fn get_messages(&mut self, board: &str, last_id: i64) -> Result<Vec<HttpB3Message>> {
        let url = format!(
            "{}/boards/{}/messages?last_id={}",
            self.base_url, board, last_id
        );

        let response = self.add_auth_header(self.client.get(&url)).send().await?;

        if !response.status().is_success() {
            anyhow::bail!("Failed to get messages: HTTP {}", response.status());
        }

        let get_response: GetMessagesResponse = response.json().await?;

        let mut result = Vec::new();

        // NOTE: POTENTIAL OPTIMIZATION
        // S3 downloads are currently sequential. For better performance with many messages,
        // these could be parallelized using futures::join_all or similar techniques.
        // With pre-signed URLs already available, there's no dependency between downloads.

        for msg in get_response.messages {
            let message_bytes = match msg.content_type {
                ContentTypeDto::Inline { message } => {
                    // Decode base64
                    base64::prelude::Engine::decode(&base64::prelude::BASE64_STANDARD, message)?
                }
                ContentTypeDto::S3 { key: _ } => {
                    // Use pre-signed download URL from response
                    let download_url = msg
                        .download_url
                        .ok_or_else(|| anyhow::anyhow!("S3 message missing download_url"))?;

                    // Download from S3 using pre-signed URL
                    let s3_response = self.client.get(&download_url).send().await?;

                    if !s3_response.status().is_success() {
                        anyhow::bail!("Failed to download from S3: HTTP {}", s3_response.status());
                    }

                    s3_response.bytes().await?.to_vec()
                }
            };

            let id: i64 = msg.id.parse()?;

            result.push(HttpB3Message::new(
                id,
                message_bytes,
                "1".to_string(),
                msg.sender_pk,
                msg.statement_kind,
                msg.batch,
                msg.mix_number,
            ));
        }

        Ok(result)
    }

    async fn insert_messages(&mut self, board: &str, messages: Vec<Message>) -> Result<()> {
        for message in messages {
            self.post_message_to_board(board, &message).await?;
        }
        Ok(())
    }
}

/// Factory for creating HttpB3 board clients.
///
/// The access token is wrapped in `Arc<RwLock<>>` so that all clones
/// (including those held by `SessionSet` tasks) share the same token
/// and see updates from the master loop.
#[derive(Clone)]
pub struct HttpB3BoardParams {
    client: reqwest::Client,
    base_url: String,
    s3_client: aws_sdk_s3::Client,
    bucket_name: String,
    access_token: Arc<RwLock<String>>,
}

impl HttpB3BoardParams {
    pub async fn new(base_url: &str, access_token: String) -> HttpB3BoardParams {
        use sequent_core::types::env_vars as ev;
        let s3_endpoint = std::env::var(ev::AWS_ENDPOINT_URL)
            .unwrap_or_else(|_| ev::DEFAULT_S3_ENDPOINT.to_string());
        let bucket_name =
            std::env::var(ev::S3_BUCKET_NAME).unwrap_or_else(|_| ev::DEFAULT_S3_BUCKET.to_string());

        // Use explicit credentials for LocalStack (avoids IMDS calls)
        let creds =
            aws_sdk_s3::config::Credentials::new("test", "test", None, None, "static-credentials");

        let config = aws_config::defaults(aws_config::BehaviorVersion::latest())
            .credentials_provider(creds)
            .region(ev::DEFAULT_AWS_REGION)
            .load()
            .await;

        let s3_config = aws_sdk_s3::config::Builder::from(&config)
            .endpoint_url(&s3_endpoint)
            .force_path_style(true)
            .build();
        let s3_client = aws_sdk_s3::Client::from_conf(s3_config);

        HttpB3BoardParams {
            client: reqwest::Client::new(),
            base_url: base_url.to_string(),
            s3_client,
            bucket_name,
            access_token: Arc::new(RwLock::new(access_token)),
        }
    }

    /// Updates the access token for authentication.
    ///
    /// Because the token is behind `Arc<RwLock<>>`, all clones (including
    /// those held by running `SessionSet` tasks) will see the new value.
    pub fn set_access_token(&self, access_token: String) {
        *self
            .access_token
            .write()
            .expect("access_token lock poisoned") = access_token;
    }

    /// Reads the current access token.
    fn clone_access_token(&self) -> Arc<RwLock<String>> {
        self.access_token.clone()
    }

    /// Create a board client for a specific board (helper for testing)
    pub fn create_board(&self, _board_name: &str, _store_root: Option<PathBuf>) -> HttpB3 {
        HttpB3 {
            client: reqwest::Client::new(),
            base_url: self.base_url.clone(),
            s3_client: self.s3_client.clone(),
            bucket_name: self.bucket_name.clone(),
            access_token: self.clone_access_token(),
        }
    }
}

impl BoardFactory<HttpB3> for HttpB3BoardParams {
    fn get_board(&self) -> HttpB3 {
        // Board name will be set when used with Session
        HttpB3 {
            client: reqwest::Client::new(),
            base_url: self.base_url.clone(),
            s3_client: self.s3_client.clone(),
            bucket_name: self.bucket_name.clone(),
            access_token: self.clone_access_token(),
        }
    }
}

impl BoardFactoryMulti<HttpB3> for HttpB3BoardParams {
    fn get_board(&self) -> HttpB3 {
        HttpB3 {
            client: reqwest::Client::new(),
            base_url: self.base_url.clone(),
            s3_client: self.s3_client.clone(),
            bucket_name: self.bucket_name.clone(),
            access_token: self.clone_access_token(),
        }
    }
}

impl BoardMulti for HttpB3 {
    type Factory = HttpB3BoardParams;

    async fn get_messages_multi(
        &self,
        requests: &Vec<(String, i64)>,
    ) -> Result<(Vec<b4::HttpBoardMessages>, bool)> {
        use b4::api_types::{BoardMessageRequest, GetMessagesMultiRequest};

        const DEFAULT_LIMIT: i64 = 100;

        // Build the multi-board request
        let multi_req = GetMessagesMultiRequest {
            requests: requests
                .iter()
                .map(|(board, last_id)| BoardMessageRequest {
                    board: board.clone(),
                    last_id: *last_id,
                    limit: Some(DEFAULT_LIMIT),
                })
                .collect(),
        };

        // Make single HTTP POST request for all boards
        let url = format!("{}/boards/messages/multi/get", self.base_url);
        let response = self
            .add_auth_header(self.client.post(&url))
            .json(&multi_req)
            .send()
            .await?;

        if !response.status().is_success() {
            anyhow::bail!("Failed to get messages multi: HTTP {}", response.status());
        }

        let multi_response: b4::api_types::GetMessagesMultiResponse = response.json().await?;

        // Check if any board hit the limit (indicating more messages available)
        let has_more = multi_response
            .boards
            .iter()
            .any(|board_resp| board_resp.messages.len() >= DEFAULT_LIMIT as usize);

        // Process each board's messages
        let mut all_boards = Vec::new();
        for board_resp in multi_response.boards {
            let mut http_messages = Vec::new();

            for msg_with_url in board_resp.messages {
                let message_bytes = match msg_with_url.message.content_type {
                    b4::api_types::ContentType::Inline { data } => data,
                    b4::api_types::ContentType::S3 { key: _ } => {
                        // Use pre-signed download URL from response
                        let download_url = msg_with_url
                            .download_url
                            .ok_or_else(|| anyhow::anyhow!("S3 message missing download_url"))?;

                        // Download from S3 using pre-signed URL
                        let s3_response = self.client.get(&download_url).send().await?;

                        if !s3_response.status().is_success() {
                            anyhow::bail!(
                                "Failed to download from S3: HTTP {}",
                                s3_response.status()
                            );
                        }

                        s3_response.bytes().await?.to_vec()
                    }
                };

                let id: i64 = msg_with_url.message.id.parse()?;

                http_messages.push(HttpB3Message::new(
                    id,
                    message_bytes,
                    "1".to_string(),
                    msg_with_url.message.sender_pk,
                    msg_with_url.message.statement_kind,
                    msg_with_url.message.batch,
                    msg_with_url.message.mix_number,
                ));
            }

            all_boards.push(b4::HttpBoardMessages {
                board: board_resp.board,
                messages: http_messages,
            });
        }

        Ok((all_boards, has_more))
    }

    async fn insert_messages_multi(&self, requests: Vec<(String, Vec<Message>)>) -> Result<()> {
        use b4::api_types::{
            BoardConfirmRequest, BoardInitiateRequest, ConfirmMessagesMultiRequest,
            InitiateMessagesMultiRequest, MessageConfirmation, MessageMetadata,
        };
        use strand::serialization::StrandSerialize;

        if requests.is_empty() {
            return Ok(());
        }

        // Phase 1: Initiate multi-board upload - get S3 URLs for all messages
        let mut initiate_requests = Vec::new();
        let mut messages_by_board: std::collections::HashMap<String, Vec<Message>> =
            std::collections::HashMap::new();

        for (board_name, messages) in requests {
            if messages.is_empty() {
                continue;
            }

            let mut metadata_list = Vec::new();
            for message in &messages {
                let sender_pk = message.sender.pk.to_der_b64_string()?;
                let statement_kind = message.statement.get_kind().to_string();
                let batch: i32 = message.statement.get_batch_number().try_into()?;
                let mix_number: i32 = message.statement.get_mix_number().try_into()?;
                let message_bytes = message.strand_serialize()?;

                metadata_list.push(MessageMetadata {
                    size: message_bytes.len(),
                    sender_pk,
                    statement_kind,
                    batch,
                    mix_number,
                });
            }

            initiate_requests.push(BoardInitiateRequest {
                board: board_name.clone(),
                messages: metadata_list,
            });

            messages_by_board.insert(board_name, messages);
        }

        let initiate_req = InitiateMessagesMultiRequest {
            requests: initiate_requests,
        };

        let url = format!("{}/boards/messages/multi/initiate", self.base_url);
        let response = self
            .add_auth_header(self.client.post(&url))
            .json(&initiate_req)
            .send()
            .await?;

        if !response.status().is_success() {
            anyhow::bail!(
                "Failed to initiate multi-board upload: HTTP {}",
                response.status()
            );
        }

        let initiate_response: b4::api_types::InitiateMessagesMultiResponse =
            response.json().await?;

        // Phase 2: Upload messages (to S3 or prepare inline data)
        let mut confirm_requests = Vec::new();

        for board_response in initiate_response.boards {
            let board_name = &board_response.board;
            let messages = messages_by_board
                .get(board_name)
                .ok_or_else(|| anyhow::anyhow!("Missing messages for board {}", board_name))?;

            if messages.len() != board_response.uploads.len() {
                anyhow::bail!(
                    "Mismatch in message count for board {}: sent {}, got {} upload slots",
                    board_name,
                    messages.len(),
                    board_response.uploads.len()
                );
            }

            let mut confirmations = Vec::new();

            for (message, upload_info) in messages.iter().zip(board_response.uploads.iter()) {
                let message_bytes = message.strand_serialize()?;

                if upload_info.should_upload {
                    // Large message - upload to S3
                    if let Some(upload_url) = &upload_info.upload_url {
                        let s3_response = self
                            .client
                            .put(upload_url)
                            .body(message_bytes)
                            .send()
                            .await?;

                        if !s3_response.status().is_success() {
                            anyhow::bail!(
                                "Failed to upload to S3 for board {}: HTTP {}",
                                board_name,
                                s3_response.status()
                            );
                        }

                        // S3 message - no inline data in confirmation
                        confirmations.push(MessageConfirmation {
                            message_id: upload_info.message_id.clone(),
                            data: None,
                        });
                    } else {
                        anyhow::bail!("Server indicated upload needed but provided no URL");
                    }
                } else {
                    // Small message - send inline in confirmation
                    confirmations.push(MessageConfirmation {
                        message_id: upload_info.message_id.clone(),
                        data: Some(message_bytes),
                    });
                }
            }

            confirm_requests.push(BoardConfirmRequest {
                board: board_name.clone(),
                confirmations,
            });
        }

        // Phase 3: Confirm all uploads
        let confirm_req = ConfirmMessagesMultiRequest {
            requests: confirm_requests,
        };

        let url = format!("{}/boards/messages/multi/confirm", self.base_url);
        let response = self
            .add_auth_header(self.client.post(&url))
            .json(&confirm_req)
            .send()
            .await?;

        if !response.status().is_success() {
            anyhow::bail!(
                "Failed to confirm multi-board upload: HTTP {}",
                response.status()
            );
        }

        Ok(())
    }
}

use sequent_core::types::ceremonies::{HeartbeatRequest, TrusteeModePolicy};

impl HttpB3BoardParams {
    /// Send a heartbeat to B4 for the given board.
    #[tracing::instrument(skip(self), fields(board_name, trustee_name), err)]
    pub async fn send_heartbeat(
        &self,
        board_name: &str,
        sender_pk: &str,
        trustee_name: &str,
        trustee_mode: TrusteeModePolicy,
    ) -> anyhow::Result<()> {
        let url = format!("{}/boards/{}/sessions/heartbeat", self.base_url, board_name);
        let access_token = self
            .access_token
            .read()
            .expect("access_token lock poisoned")
            .clone();

        let resp = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {access_token}"))
            .json(&HeartbeatRequest {
                board_name: board_name.to_string(),
                sender_pk: sender_pk.to_string(),
                trustee_name: trustee_name.to_string(),
                trustee_mode,
            })
            .send()
            .await?;

        if !resp.status().is_success() {
            anyhow::bail!("Heartbeat failed: HTTP {}", resp.status());
        }
        Ok(())
    }
}

/// HTTP client for bulletin board index (list of boards)
pub struct HttpB3Index {
    client: reqwest::Client,
    base_url: String,
    access_token: String,
}

impl HttpB3Index {
    pub fn new(base_url: &str, access_token: String) -> HttpB3Index {
        HttpB3Index {
            client: reqwest::Client::new(),
            base_url: base_url.to_string(),
            access_token,
        }
    }

    /// Adds the Authorization header for B4 authentication
    fn add_auth_header(&self, request: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        request.header("Authorization", format!("Bearer {}", self.access_token))
    }

    pub async fn get_boards(&self) -> Result<Vec<String>> {
        #[derive(Deserialize)]
        struct BoardInfo {
            name: String,
        }

        #[derive(Deserialize)]
        struct BoardsResponse {
            boards: Vec<BoardInfo>,
        }

        let url = format!("{}/boards", self.base_url);
        let response = self
            .add_auth_header(self.client.get(&url))
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("While sending the request: {e:?}"))?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "Failed to get boards: HTTP {}",
                response.status()
            ));
        }
        let boards_response: BoardsResponse = response.json().await?;
        Ok(boards_response.boards.into_iter().map(|b| b.name).collect())
    }
}
