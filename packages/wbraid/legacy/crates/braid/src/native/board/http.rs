use anyhow::Result;
use serde::Deserialize;
use std::path::PathBuf;
use tracing::info;

use b4::api_types::{
    InitiateMessageRequest, InitiateMessageResponse, ConfirmMessageRequest,
    GetMessagesResponse, ContentType,
};
use b4::HttpB4Message;
use cryptography::context::Context;

use crate::protocol::board::{Board, BoardFactory, BoardFactoryMulti, BoardMulti};

/// HTTP client for bulletin board using Service API
pub struct HttpB4 {
    client: reqwest::Client,
    base_url: String,
}

impl HttpB4 {
    pub async fn new(base_url: &str) -> HttpB4 {
        HttpB4 {
            client: reqwest::Client::new(),
            base_url: base_url.to_string(),
        }
    }

    /// Helper to post a single HttpB4Message to a specific board
    async fn post_http_message_to_board(&self, board: &str, http_message: &HttpB4Message) -> Result<()> {
        // Message is already serialized in HttpB4Message
        let message_bytes = &http_message.message;
        let size = message_bytes.len();

        // Phase 1: Initiate message
        let initiate_url = format!("{}/boards/{}/messages/initiate", self.base_url, board);
        let initiate_req = InitiateMessageRequest {
            size,
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
                    version: http_message.version.clone(),
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
                data: Some(message_bytes.clone()),
                version: http_message.version.clone(),
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

        Ok(())
    }
}

impl<C: Context> Board<C> for HttpB4 {
    type Factory = HttpB4BoardParams;
    
    async fn get_messages(&mut self, board: &str, last_id: i64) -> Result<Vec<HttpB4Message>> {
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

        let get_response: GetMessagesResponse = response.json().await?;

        let mut result = Vec::new();

        // NOTE: POTENTIAL OPTIMIZATION
        // S3 downloads are currently sequential. For better performance with many messages,
        // these could be parallelized using futures::join_all or similar techniques.
        // With pre-signed URLs already available, there's no dependency between downloads.

        for msg in get_response.messages {
            let message_bytes = match msg.message.content_type {
                ContentType::Inline { data } => {
                    // Already decoded by ContentType's Deserialize impl
                    data
                }
                ContentType::S3 { key: _ } => {
                    // Use pre-signed download URL from response
                    let download_url = msg.download_url
                        .ok_or_else(|| anyhow::anyhow!("S3 message missing download_url"))?;
                    
                    // Download from S3 using pre-signed URL
                    let s3_response = self
                        .client
                        .get(&download_url)
                        .send()
                        .await?;
                    
                    if !s3_response.status().is_success() {
                        anyhow::bail!("Failed to download from S3: HTTP {}", s3_response.status());
                    }
                    
                    s3_response.bytes().await?.to_vec()
                }
            };

            let id: i64 = msg.message.id.parse()?;

            result.push(HttpB4Message::new(
                id,
                message_bytes,
                msg.message.version,
            ));
        }

        Ok(result)
    }

    async fn post_messages(&mut self, board: &str, messages: Vec<HttpB4Message>) -> Result<()> {
        for http_message in messages {
            self.post_http_message_to_board(board, &http_message).await?;
        }
        Ok(())
    }
}

/// Factory for creating HttpB4 board clients
#[derive(Clone)]
pub struct HttpB4BoardParams {
    pub base_url: String,
}

impl HttpB4BoardParams {
    pub fn new(base_url: &str) -> HttpB4BoardParams {
        HttpB4BoardParams {
            base_url: base_url.to_string(),
        }
    }
    
    /// Create a board client for a specific board (helper for testing)
    pub fn create_board(&self, _board_name: &str, _store_root: Option<PathBuf>) -> HttpB4 {
        HttpB4 {
            client: reqwest::Client::new(),
            base_url: self.base_url.clone(),
        }
    }
}

impl<C: Context> BoardFactory<C, HttpB4> for HttpB4BoardParams {
    fn get_board(&self) -> HttpB4 {
        HttpB4 {
            client: reqwest::Client::new(),
            base_url: self.base_url.clone(),
        }
    }
}

impl<C: Context> BoardFactoryMulti<C, HttpB4> for HttpB4BoardParams {
    fn get_board(&self) -> HttpB4 {
        HttpB4 {
            client: reqwest::Client::new(),
            base_url: self.base_url.clone(),
        }
    }
}

impl<C: Context> BoardMulti<C> for HttpB4 {
    type Factory = HttpB4BoardParams;

    async fn get_messages_multi(
        &self,
        requests: &Vec<(String, i64)>,
    ) -> Result<(Vec<b4::HttpBoardMessages>, bool)> {
        use b4::api_types::{GetMessagesMultiRequest, BoardMessageRequest};
        
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
        let response = self.client
            .post(&url)
            .json(&multi_req)
            .send()
            .await?;
        
        if !response.status().is_success() {
            anyhow::bail!("Failed to get messages multi: HTTP {}", response.status());
        }
        
        let multi_response: b4::api_types::GetMessagesMultiResponse = response.json().await?;
        
        // Check if any board hit the limit (indicating more messages available)
        let has_more = multi_response.boards.iter()
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
                        let download_url = msg_with_url.download_url
                            .ok_or_else(|| anyhow::anyhow!("S3 message missing download_url"))?;
                        
                        // Download from S3 using pre-signed URL
                        let s3_response = self
                            .client
                            .get(&download_url)
                            .send()
                            .await?;
                        
                        if !s3_response.status().is_success() {
                            anyhow::bail!("Failed to download from S3: HTTP {}", s3_response.status());
                        }
                        
                        s3_response.bytes().await?.to_vec()
                    }
                };
                
                let id: i64 = msg_with_url.message.id.parse()?;
                
                http_messages.push(HttpB4Message::new(
                    id,
                    message_bytes,
                    msg_with_url.message.version,
                ));
            }
            
            all_boards.push(b4::HttpBoardMessages {
                board: board_resp.board,
                messages: http_messages,
            });
        }
        
        Ok((all_boards, has_more))
    }

    async fn post_messages_multi(&self, requests: Vec<(String, Vec<HttpB4Message>)>) -> Result<()> {
        use b4::api_types::{
            InitiateMessagesMultiRequest, BoardInitiateRequest, MessageMetadata,
            ConfirmMessagesMultiRequest, BoardConfirmRequest, MessageConfirmation,
        };
        if requests.is_empty() {
            return Ok(());
        }
        
        // Phase 1: Initiate multi-board upload - get S3 URLs for all messages
        let mut initiate_requests = Vec::new();
        let mut messages_by_board: std::collections::HashMap<String, Vec<HttpB4Message>> = std::collections::HashMap::new();
        
        for (board_name, messages) in requests {
            if messages.is_empty() {
                continue;
            }
            
            let mut metadata_list = Vec::new();
            for http_message in &messages {
                metadata_list.push(MessageMetadata {
                    size: http_message.message.len(),
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
        let response = self.client
            .post(&url)
            .json(&initiate_req)
            .send()
            .await?;
        
        if !response.status().is_success() {
            anyhow::bail!("Failed to initiate multi-board upload: HTTP {}", response.status());
        }
        
        let initiate_response: b4::api_types::InitiateMessagesMultiResponse = response.json().await?;
        
        // Phase 2: Upload messages (to S3 or prepare inline data)
        let mut confirm_requests = Vec::new();
        
        for board_response in initiate_response.boards {
            let board_name = &board_response.board;
            let messages = messages_by_board.get(board_name)
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
            
            for (http_message, upload_info) in messages.iter().zip(board_response.uploads.iter()) {
                let message_bytes = &http_message.message;
                
                if upload_info.should_upload {
                    // Large message - upload to S3
                    if let Some(upload_url) = &upload_info.upload_url {
                        let s3_response = self.client
                            .put(upload_url)
                            .body(message_bytes.clone())
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
                            version: http_message.version.clone(),
                        });
                    } else {
                        anyhow::bail!("Server indicated upload needed but provided no URL");
                    }
                } else {
                    // Small message - send inline in confirmation
                    confirmations.push(MessageConfirmation {
                        message_id: upload_info.message_id.clone(),
                        data: Some(message_bytes.clone()),
                        version: http_message.version.clone(),
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
        let response = self.client
            .post(&url)
            .json(&confirm_req)
            .send()
            .await?;
        
        if !response.status().is_success() {
            anyhow::bail!("Failed to confirm multi-board upload: HTTP {}", response.status());
        }
        
        Ok(())
    }
}

/// HTTP client for bulletin board index (list of boards)
pub struct HttpB4Index {
    client: reqwest::Client,
    base_url: String,
}

impl HttpB4Index {
    pub fn new(base_url: &str) -> HttpB4Index {
        HttpB4Index {
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
