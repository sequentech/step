// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! HTTP+S3 transport (§8, M2): the braid-side client that talks to a live b4 over
//! HTTP, implementing [`Transport`].
//!
//! Posting uses b4's two-step flow (`initiate` → S3 `PUT` or inline → `confirm`);
//! fetching pulls all of a board's messages and, for S3-backed bodies, downloads
//! them via the presigned URL b4 returns. b4 is a dumb opaque store (§8), so this
//! client (de)serializes `ProtocolMessage<C>` itself and enforces the version
//! exact-match at the boundary (§10.1). Verification happens later in the board
//! client — the transport only moves bytes.

use anyhow::{anyhow, bail, Result};
use async_trait::async_trait;

use cryptography::context::Context;
use cryptography::utils::serialization::{VDeserializable, VSerializable};

use crate::messages::wire::{schema_version, MessageType, ProtocolMessage};
use b4::api_types::{
    ConfirmMessageRequest, ContentType, CreateBoardRequest, GetBlobsResponse,
    InitiateMessageRequest, InitiateMessageResponse,
};

use crate::board::transport::Transport;

/// HTTP client for a single b4 board. Not parameterized by `C`: it moves opaque
/// bytes, and the `Transport<C>` impl (de)serializes per `C`.
pub struct HttpTransport {
    client: reqwest::Client,
    base_url: String,
    board: String,
}

impl HttpTransport {
    pub fn new(base_url: &str, board: &str) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: base_url.trim_end_matches('/').to_string(),
            board: board.to_string(),
        }
    }

    /// Create the board on b4. Board creation is untrusted namespacing (§8.3): a
    /// duplicate/failed create is an availability concern only.
    pub async fn create_board(base_url: &str, board: &str) -> Result<()> {
        let client = reqwest::Client::new();
        let url = format!("{}/boards", base_url.trim_end_matches('/'));
        let resp = client
            .post(&url)
            .json(&CreateBoardRequest {
                name: board.to_string(),
            })
            .send()
            .await?;
        if !resp.status().is_success() {
            bail!("failed to create board {}: HTTP {}", board, resp.status());
        }
        Ok(())
    }

    /// Fetch every message on the board as raw bytes, enforcing the version
    /// exact-match (§10.1). S3-backed bodies are downloaded via the presigned URL.
    async fn fetch_raw(&self) -> Result<Vec<Vec<u8>>> {
        let url = format!("{}/boards/{}/messages", self.base_url, self.board);
        let resp = self.client.get(&url).send().await?;
        if !resp.status().is_success() {
            bail!("failed to fetch messages: HTTP {}", resp.status());
        }
        let body: GetBlobsResponse = resp.json().await?;

        let expected_version = schema_version();
        let mut out = Vec::with_capacity(body.messages.len());
        for m in body.messages {
            if m.message.version != expected_version {
                bail!(
                    "version mismatch (§10.1): message {} has version {:?}, expected {:?}",
                    m.message.id,
                    m.message.version,
                    expected_version
                );
            }
            let bytes = match m.message.content_type {
                ContentType::Inline { data } => data,
                ContentType::S3 { key: _ } => {
                    let url = m
                        .download_url
                        .ok_or_else(|| anyhow!("S3 message missing download_url"))?;
                    let s3 = self.client.get(&url).send().await?;
                    if !s3.status().is_success() {
                        bail!("failed to download S3 body: HTTP {}", s3.status());
                    }
                    s3.bytes().await?.to_vec()
                }
            };
            out.push(bytes);
        }
        Ok(out)
    }

    /// Post one message's bytes via the two-step flow (initiate → S3 PUT or
    /// inline → confirm).
    async fn post_bytes(&self, bytes: Vec<u8>) -> Result<()> {
        let version = schema_version();

        let initiate_url = format!("{}/boards/{}/messages/initiate", self.base_url, self.board);
        let init: InitiateMessageResponse = {
            let resp = self
                .client
                .post(&initiate_url)
                .json(&InitiateMessageRequest { size: bytes.len() })
                .send()
                .await?;
            if !resp.status().is_success() {
                bail!("initiate failed: HTTP {}", resp.status());
            }
            resp.json().await?
        };

        let confirm_url = format!(
            "{}/boards/{}/messages/{}/confirm",
            self.base_url, self.board, init.message_id
        );

        if init.should_upload {
            let upload_url = init
                .upload_url
                .ok_or_else(|| anyhow!("server asked to upload but gave no URL"))?;
            let put = self.client.put(&upload_url).body(bytes).send().await?;
            if !put.status().is_success() {
                bail!("S3 upload failed: HTTP {}", put.status());
            }
            let confirm = self
                .client
                .post(&confirm_url)
                .json(&ConfirmMessageRequest {
                    data: None,
                    version,
                })
                .send()
                .await?;
            if !confirm.status().is_success() {
                bail!("confirm (S3) failed: HTTP {}", confirm.status());
            }
        } else {
            let confirm = self
                .client
                .post(&confirm_url)
                .json(&ConfirmMessageRequest {
                    data: Some(bytes),
                    version,
                })
                .send()
                .await?;
            if !confirm.status().is_success() {
                bail!("confirm (inline) failed: HTTP {}", confirm.status());
            }
        }
        Ok(())
    }
}

#[async_trait(?Send)]
impl<C: Context> Transport<C> for HttpTransport {
    async fn fetch_configuration(&self) -> Result<ProtocolMessage<C>> {
        for bytes in self.fetch_raw().await? {
            let wm = ProtocolMessage::<C>::deser(&bytes)
                .map_err(|e| anyhow!("failed to deserialize wire message: {:?}", e))?;
            if wm.message_type == MessageType::Configuration {
                return Ok(wm);
            }
        }
        bail!("board {} has no Configuration message", self.board)
    }

    async fn fetch(&self) -> Result<Vec<ProtocolMessage<C>>> {
        let mut out = Vec::new();
        for bytes in self.fetch_raw().await? {
            let wm = ProtocolMessage::<C>::deser(&bytes)
                .map_err(|e| anyhow!("failed to deserialize wire message: {:?}", e))?;
            if wm.message_type != MessageType::Configuration {
                out.push(wm);
            }
        }
        Ok(out)
    }

    async fn post(&self, messages: Vec<ProtocolMessage<C>>) -> Result<()> {
        for message in &messages {
            self.post_bytes(message.ser()).await?;
        }
        Ok(())
    }
}
