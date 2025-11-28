// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tracing::info;

use b4::messages::message::Message;
use b4::HttpB3Message;
use strand::serialization::StrandSerialize;

const MAX_INLINE_MESSAGE_SIZE: usize = 1024 * 1024; // 1MB

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
struct ListMessagesResponse {
    messages: Vec<MessageRow>,
}

#[derive(Debug, Deserialize)]
struct MessageRow {
    id: String,
    timestamp: i64,
    size: usize,
    content_type: ContentTypeDto,
    sender_pk: String,
    statement_kind: String,
    batch: i32,
    mix_number: i32,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum ContentTypeDto {
    Inline { message: String },  // base64 encoded
    S3 { key: String },
}

/// HTTP client for bulletin board using Service API
pub struct HttpB3 {
    client: reqwest::Client,
    base_url: String,
    board_name: String,
    s3_client: aws_sdk_s3::Client,
    bucket_name: String,
    store_root: Option<PathBuf>,
}

impl HttpB3 {
    pub async fn new(
        base_url: &str,
        board_name: &str,
        s3_client: aws_sdk_s3::Client,
        bucket_name: &str,
        store_root: Option<PathBuf>,
    ) -> HttpB3 {
        HttpB3 {
            client: reqwest::Client::new(),
            base_url: base_url.to_string(),
            board_name: board_name.to_string(),
            s3_client,
            bucket_name: bucket_name.to_string(),
            store_root,
        }
    }

    /// Helper to process message rows from list response
    async fn process_message_rows(&self, rows: Vec<MessageRow>) -> Result<Vec<HttpB3Message>> {
        let mut result = Vec::new();

        for msg_row in rows {
            let message_bytes = match msg_row.content_type {
                ContentTypeDto::Inline { message } => {
                    base64::prelude::Engine::decode(&base64::prelude::BASE64_STANDARD, message)?
                }
                ContentTypeDto::S3 { key } => {
                    let obj = self
                        .s3_client
                        .get_object()
                        .bucket(&self.bucket_name)
                        .key(&key)
                        .send()
                        .await?;

                    let bytes = obj.body.collect().await?;
                    bytes.to_vec()
                }
            };

            let id: i64 = msg_row.id.parse()?;

            result.push(HttpB3Message::new(
                id,
                message_bytes,
                "1".to_string(),
                msg_row.sender_pk,
                msg_row.statement_kind,
                msg_row.batch,
                msg_row.mix_number,
            ));
        }

        Ok(result)
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
            .client
            .post(&initiate_url)
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
                self.client
                    .put(upload_url)
                    .body(message_bytes.clone())
                    .send()
                    .await?;
            }
        }

        // Phase 3: Confirm message
        let confirm_url = format!(
            "{}/boards/{}/messages/{}/confirm",
            self.base_url, board, init_resp.message_id
        );

        let confirm_req = ConfirmMessageRequest {
            data: if !init_resp.should_upload {
                Some(message_bytes)
            } else {
                None
            },
            sender_pk,
            statement_kind,
            batch,
            mix_number,
        };

        let confirm_response = self
            .client
            .post(&confirm_url)
            .json(&confirm_req)
            .send()
            .await?;

        if !confirm_response.status().is_success() {
            anyhow::bail!(
                "Failed to confirm message: HTTP {}",
                confirm_response.status()
            );
        }

        Ok(())
    }
}

impl super::Board for HttpB3 {
    type Factory = HttpB3BoardParams;
    
    async fn get_messages(&mut self, board: &str, last_id: i64) -> Result<Vec<HttpB3Message>> {
        let url = format!(
            "{}/boards/{}/messages?last_id={}",
            self.base_url, board, last_id
        );

        let response = self
            .client
            .get(&url)
            .send()
            .await?;

        if !response.status().is_success() {
            anyhow::bail!("Failed to get messages: HTTP {}", response.status());
        }

        let list_response: ListMessagesResponse = response.json().await?;

        let mut result = Vec::new();

        for msg_row in list_response.messages {
            let message_bytes = match msg_row.content_type {
                ContentTypeDto::Inline { message } => {
                    // Decode base64
                    base64::prelude::Engine::decode(&base64::prelude::BASE64_STANDARD, message)?
                }
                ContentTypeDto::S3 { key } => {
                    // Download from S3
                    let obj = self
                        .s3_client
                        .get_object()
                        .bucket(&self.bucket_name)
                        .key(&key)
                        .send()
                        .await?;

                    let bytes = obj.body.collect().await?;
                    bytes.to_vec()
                }
            };

            let id: i64 = msg_row.id.parse()?;

            result.push(HttpB3Message::new(
                id,
                message_bytes,
                "1".to_string(),
                msg_row.sender_pk,
                msg_row.statement_kind,
                msg_row.batch,
                msg_row.mix_number,
            ));
        }

        Ok(result)
    }

    async fn insert_messages(&mut self, board: &str, messages: Vec<Message>) -> Result<()> {
        for message in messages {
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
                .client
                .post(&initiate_url)
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
                        .client
                        .post(&confirm_url)
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
                    .client
                    .post(&confirm_url)
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
        }

        Ok(())
    }
}

/// Factory for creating HttpB3 board clients
#[derive(Clone)]
pub struct HttpB3BoardParams {
    base_url: String,
    s3_client: aws_sdk_s3::Client,
    bucket_name: String,
}

impl HttpB3BoardParams {
    pub async fn new(base_url: &str) -> HttpB3BoardParams {
        // Read S3 configuration from environment variables
        let s3_endpoint = std::env::var("AWS_ENDPOINT_URL")
            .unwrap_or_else(|_| "http://localhost:4566".to_string());
        let bucket_name = std::env::var("S3_BUCKET_NAME")
            .unwrap_or_else(|_| "wbraid-messages".to_string());
        
        // Use explicit credentials for LocalStack (avoids IMDS calls)
        let creds = aws_sdk_s3::config::Credentials::new(
            "test",
            "test",
            None,
            None,
            "static-credentials"
        );
        
        let config = aws_config::defaults(aws_config::BehaviorVersion::latest())
            .credentials_provider(creds)
            .region("us-east-1")
            .load()
            .await;
            
        let s3_config = aws_sdk_s3::config::Builder::from(&config)
            .endpoint_url(&s3_endpoint)
            .force_path_style(true)
            .build();
        let s3_client = aws_sdk_s3::Client::from_conf(s3_config);

        HttpB3BoardParams {
            base_url: base_url.to_string(),
            s3_client,
            bucket_name,
        }
    }
    
    /// Create a board client for a specific board (helper for testing)
    pub fn create_board(&self, board_name: &str, store_root: Option<PathBuf>) -> HttpB3 {
        HttpB3 {
            client: reqwest::Client::new(),
            base_url: self.base_url.clone(),
            board_name: board_name.to_string(),
            s3_client: self.s3_client.clone(),
            bucket_name: self.bucket_name.clone(),
            store_root,
        }
    }
}

impl super::BoardFactory<HttpB3> for HttpB3BoardParams {
    fn get_board(&self) -> HttpB3 {
        // Board name will be set when used with Session
        HttpB3 {
            client: reqwest::Client::new(),
            base_url: self.base_url.clone(),
            board_name: String::new(),
            s3_client: self.s3_client.clone(),
            bucket_name: self.bucket_name.clone(),
            store_root: None,
        }
    }
}

impl super::BoardFactoryMulti<HttpB3> for HttpB3BoardParams {
    fn get_board(&self) -> HttpB3 {
        HttpB3 {
            client: reqwest::Client::new(),
            base_url: self.base_url.clone(),
            board_name: String::new(),
            s3_client: self.s3_client.clone(),
            bucket_name: self.bucket_name.clone(),
            store_root: None,
        }
    }
}

impl super::BoardMulti for HttpB3 {
    type Factory = HttpB3BoardParams;

    async fn get_messages_multi(
        &self,
        requests: &Vec<(String, i64)>,
    ) -> Result<(Vec<b4::HttpBoardMessages>, bool)> {
        let mut all_boards = Vec::new();
        
        for (board_name, last_id) in requests {
            let url = format!("{}/boards/{}/messages?last_id={}", 
                self.base_url, board_name, last_id);
            
            let response = self.client.get(&url).send().await?;
            let list_response: ListMessagesResponse = response.json().await?;
            
            let messages = self.process_message_rows(list_response.messages).await?;
            
            all_boards.push(b4::HttpBoardMessages {
                board: board_name.clone(),
                messages,
            });
        }
        
        // For HTTP, we don't have server-side truncation like gRPC
        Ok((all_boards, false))
    }

    async fn insert_messages_multi(&self, requests: Vec<(String, Vec<Message>)>) -> Result<()> {
        for (board_name, messages) in requests {
            if !messages.is_empty() {
                for message in messages {
                    self.post_message_to_board(&board_name, &message).await?;
                }
            }
        }
        Ok(())
    }
}

/// HTTP client for bulletin board index (list of boards)
pub struct HttpB3Index {
    client: reqwest::Client,
    base_url: String,
}

impl HttpB3Index {
    pub fn new(base_url: &str) -> HttpB3Index {
        HttpB3Index {
            client: reqwest::Client::new(),
            base_url: base_url.to_string(),
        }
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
        let response = self.client.get(&url).send().await?;
        let boards_response: BoardsResponse = response.json().await?;
        
        Ok(boards_response.boards.into_iter().map(|b| b.name).collect())
    }
}
