// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Browser HTTP+S3 transport (§8, M3): the wasm-side client that talks to a live
//! b4 over the browser `fetch` API, implementing [`Transport`].
//!
//! This is the wasm counterpart of the native [`HttpTransport`](crate::native::http_transport):
//! the protocol logic is identical — the two-step post (`initiate` → S3 `PUT` or
//! inline → `confirm`), a full fetch-all each update, and the §10.1 version
//! exact-match at the boundary — only the HTTP mechanism differs (`web_sys`
//! `fetch` + `JsFuture` instead of `reqwest`). b4 is a dumb opaque store (§8), so
//! this client (de)serializes `ProtocolMessage<C>` itself; verification happens later
//! in the board client.
//!
//! The `fetch` futures are `!Send`, which is why the [`Transport`] seam is `?Send`
//! (spec Option B). Runs on the page's main thread (`window`).

use anyhow::{anyhow, bail, Result};
use async_trait::async_trait;
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;
use web_sys::{Request, RequestInit, RequestMode, Response};

use cryptography::context::Context;
use cryptography::utils::serialization::{VDeserializable, VSerializable};

use crate::messages::wire::{schema_version, MessageType, ProtocolMessage};
use b4::api_types::{
    ConfirmMessageRequest, ContentType, CreateBoardRequest, GetBlobsResponse,
    InitiateMessageRequest, InitiateMessageResponse,
};

use crate::board::transport::Transport;

/// Browser `fetch` client for a single b4 board. Not parameterized by `C`: it
/// moves opaque bytes, and the `Transport<C>` impl (de)serializes per `C`.
pub struct WasmHttpTransport {
    base_url: String,
    board: String,
}

impl WasmHttpTransport {
    pub fn new(base_url: &str, board: &str) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            board: board.to_string(),
        }
    }

    /// Create the board on b4. Board creation is untrusted namespacing (§8.3): a
    /// duplicate/failed create is an availability concern only.
    pub async fn create_board(base_url: &str, board: &str) -> Result<()> {
        let url = format!("{}/boards", base_url.trim_end_matches('/'));
        let body = serde_json::to_string(&CreateBoardRequest {
            name: board.to_string(),
        })?;
        let resp = fetch_with_body("POST", &url, Some(&body))
            .await
            .map_err(js_err)?;
        if !resp.ok() {
            bail!("failed to create board {}: HTTP {}", board, resp.status());
        }
        Ok(())
    }

    /// Fetch every message on the board as raw bytes, enforcing the version
    /// exact-match (§10.1). S3-backed bodies are downloaded via the presigned URL.
    async fn fetch_raw(&self) -> Result<Vec<Vec<u8>>> {
        let url = format!("{}/boards/{}/messages", self.base_url, self.board);
        let resp = fetch_with_body("GET", &url, None).await.map_err(js_err)?;
        if !resp.ok() {
            bail!("failed to fetch messages: HTTP {}", resp.status());
        }
        let json = JsFuture::from(resp.json().map_err(js_err)?)
            .await
            .map_err(js_err)?;
        let body: GetBlobsResponse = serde_wasm_bindgen::from_value(json)
            .map_err(|e| anyhow!("failed to parse messages response: {e}"))?;

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
                    let resp = fetch_with_body("GET", &url, None).await.map_err(js_err)?;
                    if !resp.ok() {
                        bail!("failed to download S3 body: HTTP {}", resp.status());
                    }
                    response_bytes(&resp).await.map_err(js_err)?
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
        let init_body = serde_json::to_string(&InitiateMessageRequest { size: bytes.len() })?;
        let resp = fetch_with_body("POST", &initiate_url, Some(&init_body))
            .await
            .map_err(js_err)?;
        if !resp.ok() {
            bail!("initiate failed: HTTP {}", resp.status());
        }
        let json = JsFuture::from(resp.json().map_err(js_err)?)
            .await
            .map_err(js_err)?;
        let init: InitiateMessageResponse = serde_wasm_bindgen::from_value(json)
            .map_err(|e| anyhow!("failed to parse initiate response: {e}"))?;

        let confirm_url = format!(
            "{}/boards/{}/messages/{}/confirm",
            self.base_url, self.board, init.message_id
        );

        if init.should_upload {
            let upload_url = init
                .upload_url
                .ok_or_else(|| anyhow!("server asked to upload but gave no URL"))?;
            let put = fetch_put_bytes(&upload_url, &bytes).await.map_err(js_err)?;
            if !put.ok() {
                bail!("S3 upload failed: HTTP {}", put.status());
            }
            let confirm_body = serde_json::to_string(&ConfirmMessageRequest {
                data: None,
                version,
            })?;
            let confirm = fetch_with_body("POST", &confirm_url, Some(&confirm_body))
                .await
                .map_err(js_err)?;
            if !confirm.ok() {
                bail!("confirm (S3) failed: HTTP {}", confirm.status());
            }
        } else {
            let confirm_body = serde_json::to_string(&ConfirmMessageRequest {
                data: Some(bytes),
                version,
            })?;
            let confirm = fetch_with_body("POST", &confirm_url, Some(&confirm_body))
                .await
                .map_err(js_err)?;
            if !confirm.ok() {
                bail!("confirm (inline) failed: HTTP {}", confirm.status());
            }
        }
        Ok(())
    }
}

#[async_trait(?Send)]
impl<C: Context> Transport<C> for WasmHttpTransport {
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

///////////////////////////////////////////////////////////////////////////
// web_sys fetch helpers
///////////////////////////////////////////////////////////////////////////

/// Convert a `JsValue` error into an `anyhow::Error` (stringified immediately, so
/// the `!Send` `JsValue` is not carried).
fn js_err(e: JsValue) -> anyhow::Error {
    anyhow!("{:?}", e)
}

/// `fetch` with an optional JSON string body (`Content-Type: application/json`).
async fn fetch_with_body(method: &str, url: &str, body: Option<&str>) -> Result<Response, JsValue> {
    let opts = RequestInit::new();
    opts.set_method(method);
    opts.set_mode(RequestMode::Cors);
    if let Some(b) = body {
        opts.set_body(&JsValue::from_str(b));
    }
    let request = Request::new_with_str_and_init(url, &opts)?;
    if body.is_some() {
        request.headers().set("Content-Type", "application/json")?;
    }
    let window = web_sys::window().ok_or_else(|| JsValue::from_str("no window"))?;
    let resp_value = JsFuture::from(window.fetch_with_request(&request)).await?;
    resp_value.dyn_into::<Response>()
}

/// `PUT` raw bytes (used for the S3 presigned upload).
async fn fetch_put_bytes(url: &str, bytes: &[u8]) -> Result<Response, JsValue> {
    let opts = RequestInit::new();
    opts.set_method("PUT");
    opts.set_mode(RequestMode::Cors);
    let array = js_sys::Uint8Array::from(bytes);
    opts.set_body(&array);
    let request = Request::new_with_str_and_init(url, &opts)?;
    let window = web_sys::window().ok_or_else(|| JsValue::from_str("no window"))?;
    let resp_value = JsFuture::from(window.fetch_with_request(&request)).await?;
    resp_value.dyn_into::<Response>()
}

/// Read a response body as raw bytes.
async fn response_bytes(resp: &Response) -> Result<Vec<u8>, JsValue> {
    let array_buffer = JsFuture::from(resp.array_buffer()?).await?;
    Ok(js_sys::Uint8Array::new(&array_buffer).to_vec())
}
