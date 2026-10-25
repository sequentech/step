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
use cryptography::utils::serialization::{Deserializable, Serializable};

use crate::messages::wire::{schema_version, MessageType, ProtocolMessage};
use b4::api_types::{
    ConfirmMessageRequest, ContentType, CreateBoardRequest, GetBlobsResponse,
    InitiateMessageRequest, InitiateMessageResponse,
};

use crate::board::transport::{StagedRef, Transport};

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
    pub async fn fetch_raw(&self) -> Result<Vec<Vec<u8>>> {
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

    /// Export with explicit limits. Existing protocol callers retain their
    /// original fetch behavior; operator exports bound metadata and total raw
    /// message bytes before buffering each response.
    pub async fn fetch_raw_bounded(
        &self,
        metadata_bytes: usize,
        message_bytes: usize,
        message_count: usize,
        allowed_blob_prefix: &str,
    ) -> Result<Vec<Vec<u8>>> {
        let url = format!("{}/boards/{}/messages", self.base_url, self.board);
        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .build()?;
        let response = client.get(&url).send().await?;
        let body: GetBlobsResponse =
            serde_json::from_slice(&bounded_body(response, metadata_bytes).await?)?;
        if body.messages.len() > message_count {
            bail!("public transcript exceeds the message count limit");
        }
        let mut remaining = message_bytes;
        let mut out = Vec::with_capacity(body.messages.len());
        for message in body.messages {
            if message.message.version != schema_version() {
                bail!("public transcript message version mismatch");
            }
            let bytes = match message.message.content_type {
                ContentType::Inline { data } => data,
                ContentType::S3 { key } => {
                    let url = message
                        .download_url
                        .ok_or_else(|| anyhow!("S3 message missing download_url"))?;
                    let url = scoped_blob_url(&url, allowed_blob_prefix, &self.board, &key)?;
                    bounded_body(client.get(url).send().await?, remaining).await?
                }
            };
            remaining = remaining
                .checked_sub(bytes.len())
                .ok_or_else(|| anyhow!("public transcript exceeds the byte limit"))?;
            out.push(bytes);
        }
        Ok(out)
    }

    /// Stage bytes (§6.4): `initiate` to reserve an id and a presigned URL, then
    /// upload the body to S3. Nothing is visible on the board yet — b4 records
    /// no row until `confirm` — so a failure here has published nothing and
    /// claimed no slot.
    ///
    /// b4 only offers the S3 path for bodies above `MAX_INLINE_MESSAGE_SIZE`,
    /// which is `0`, so every real message is staged. If b4 ever answers with the
    /// inline path we refuse loudly rather than silently degrade: an inline
    /// message's bytes travel in the `confirm` request, so it could not be
    /// re-sent later from the durable record alone (§6.2 forbids keeping the
    /// body), which is precisely the guarantee staging exists to provide.
    async fn stage_bytes(&self, bytes: Vec<u8>) -> Result<StagedRef> {
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

        if !init.should_upload {
            bail!(
                "b4 offered inline storage for a {}-byte message; the outgoing mailbox \
                 requires staged (S3) bodies so a recorded post can be re-sent without \
                 retaining the body locally (§6.4)",
                bytes.len()
            );
        }
        let upload_url = init
            .upload_url
            .ok_or_else(|| anyhow!("server asked to upload but gave no URL"))?;
        let put = self.client.put(&upload_url).body(bytes).send().await?;
        if !put.status().is_success() {
            bail!("S3 upload failed: HTTP {}", put.status());
        }
        Ok(StagedRef(init.message_id))
    }

    /// Commit a staged message (§6.4): `confirm` by id. b4 reconstructs the S3
    /// key from the board name and the id, so the body is not re-sent, and this
    /// is exactly what makes a recorded post re-publishable after a restart.
    async fn commit_staged(&self, staged: &StagedRef) -> Result<()> {
        let confirm_url = format!(
            "{}/boards/{}/messages/{}/confirm",
            self.base_url, self.board, staged.0
        );
        let confirm = self
            .client
            .post(&confirm_url)
            .json(&ConfirmMessageRequest {
                data: None,
                version: schema_version(),
            })
            .send()
            .await?;
        if !confirm.status().is_success() {
            bail!("confirm failed: HTTP {}", confirm.status());
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

    async fn stage(&self, message: &ProtocolMessage<C>) -> Result<StagedRef> {
        self.stage_bytes(message.ser()).await
    }

    async fn commit(&self, staged: &StagedRef) -> Result<()> {
        self.commit_staged(staged).await
    }
}

/// Reject oversized HTTP bodies before accumulating them, including chunked
/// responses with no Content-Length. Dropping this future cancels the read.
async fn bounded_body(mut response: reqwest::Response, limit: usize) -> Result<Vec<u8>> {
    if !response.status().is_success() {
        bail!("public transcript fetch failed: HTTP {}", response.status());
    }
    if response
        .content_length()
        .is_some_and(|size| size > limit as u64)
    {
        bail!("public transcript response exceeds the byte limit");
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await? {
        if chunk.len() > limit.saturating_sub(bytes.len()) {
            bail!("public transcript response exceeds the byte limit");
        }
        bytes.extend_from_slice(&chunk);
    }
    Ok(bytes)
}

/// The untrusted listing may select only one message object in the configured
/// private bucket, under this board. Presigned query parameters are retained.
fn scoped_blob_url(raw: &str, prefix: &str, board: &str, key: &str) -> Result<reqwest::Url> {
    let allowed = reqwest::Url::parse(prefix)?;
    let url = reqwest::Url::parse(raw)?;
    let object = key
        .strip_prefix(&format!("{board}/messages/"))
        .ok_or_else(|| anyhow!("blob key is outside the selected board"))?;
    let canonical_id = object.len() == 36
        && object.bytes().enumerate().all(|(i, ch)| {
            if [8, 13, 18, 23].contains(&i) {
                ch == b'-'
            } else {
                ch.is_ascii_digit() || (b'a'..=b'f').contains(&ch)
            }
        });
    if !canonical_id
        || !matches!(allowed.scheme(), "http" | "https")
        || url.origin() != allowed.origin()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.fragment().is_some()
        || url.path() != format!("{}{object}", allowed.path())
    {
        bail!("blob URL is outside the configured board storage");
    }
    Ok(url)
}
