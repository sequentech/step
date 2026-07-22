// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! HTTP board client implementation for WASM using web_sys

use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::{Request, RequestInit, RequestMode, Response};

use crate::protocol::board::{Board, BoardFactory};
use b4::HttpB4Message;
use b4::api_types::{
    InitiateMessageRequest, InitiateMessageResponse, ConfirmMessageRequest,
    GetMessagesResponse, ContentType,
};
use cryptography::context::Context;

/// Parameters for creating a WASM HTTP board connection
#[derive(Clone)]
pub struct WasmHttpBoardParams {
    pub b4_url: String,
}

/// HTTP board client using web_sys fetch API
pub struct WasmHttpBoard {
    params: WasmHttpBoardParams,
}

impl WasmHttpBoard {
    pub fn new(params: WasmHttpBoardParams) -> Self {
        WasmHttpBoard { params }
    }

    /// Fetch messages from B4 for a specific board
    /// 
    /// Maintains all-or-nothing semantics: each HttpB4Message is only constructed
    /// after BOTH metadata (from list response) AND complete message data (inline or S3)
    /// are successfully fetched. If S3 download fails, the entire operation aborts.
    async fn fetch_messages_internal(&self, board_name: &str, last_id: i64) -> Result<Vec<HttpB4Message>, JsValue> {
        let url = format!("{}/boards/{}/messages?last_id={}", self.params.b4_url, board_name, last_id);
        
        let opts = RequestInit::new();
        opts.set_method("GET");
        opts.set_mode(RequestMode::Cors);
        
        let request = Request::new_with_str_and_init(&url, &opts)?;
        
        let window = web_sys::window().ok_or_else(|| JsValue::from_str("No window"))?;
        let resp_value = JsFuture::from(window.fetch_with_request(&request)).await?;
        let resp: Response = resp_value.dyn_into()?;
        
        if !resp.ok() {
            return Err(JsValue::from_str(&format!(
                "HTTP error: {}",
                resp.status()
            )));
        }
        
        let json = JsFuture::from(resp.json()?).await?;
        
        let get_response: GetMessagesResponse = serde_wasm_bindgen::from_value(json)
            .map_err(|e| JsValue::from_str(&format!("Failed to parse messages response: {}", e)))?;
        
        // Convert to HttpB3Message, fetching S3 content using pre-signed URLs
        let mut messages = Vec::new();
        
        // NOTE: POTENTIAL OPTIMIZATION
        // S3 downloads are currently sequential. For better performance with many messages,
        // these could be parallelized using futures::join_all or similar techniques.
        // With pre-signed URLs already available, there's no dependency between downloads.
        
        for msg in get_response.messages {
            let message_bytes = match msg.message.content_type {
                ContentType::Inline { data } => data,
                ContentType::S3 { key: _ } => {
                    // Use pre-signed download URL from response (no individual B4 requests needed!)
                    let download_url = msg.download_url
                        .ok_or_else(|| JsValue::from_str("S3 message missing download_url"))?;
                    
                    // Fetch the actual binary content from the pre-signed S3 URL
                    let opts2 = RequestInit::new();
                    opts2.set_method("GET");
                    opts2.set_mode(RequestMode::Cors);
                    
                    let request2 = Request::new_with_str_and_init(&download_url, &opts2)?;
                    let resp_value2 = JsFuture::from(window.fetch_with_request(&request2)).await?;
                    let resp2: Response = resp_value2.dyn_into()?;
                    
                    if !resp2.ok() {
                        return Err(JsValue::from_str(&format!(
                            "Failed to download S3 content: HTTP {}",
                            resp2.status()
                        )));
                    }
                    
                    let array_buffer = JsFuture::from(resp2.array_buffer()?).await?;
                    let uint8_array = js_sys::Uint8Array::new(&array_buffer);
                    uint8_array.to_vec()
                }
            };
            
            let id: i64 = msg.message.id.parse()
                .map_err(|e| JsValue::from_str(&format!("Failed to parse message ID '{}': {}", msg.message.id, e)))?;
            
            messages.push(HttpB4Message::new(
                id,
                message_bytes,
                msg.message.version,
            ));
        }
        
        Ok(messages)
    }

    /// Post a single HttpB4Message to B4
    async fn post_http_message_internal(&self, board_name: &str, http_message: HttpB4Message) -> Result<(), JsValue> {
        
        // Message is already serialized in HttpB4Message
        let message_bytes = http_message.message;
        let size = message_bytes.len();
        
        // Phase 1: Initiate message
        let initiate_url = format!("{}/boards/{}/messages/initiate", self.params.b4_url, board_name);
        
        let initiate_req = InitiateMessageRequest {
            size,
        };
        
        let body = serde_json::to_string(&initiate_req)
            .map_err(|e| JsValue::from_str(&format!("JSON error: {}", e)))?;
        
        let opts = RequestInit::new();
        opts.set_method("POST");
        opts.set_mode(RequestMode::Cors);
        opts.set_body(&JsValue::from_str(&body));
        
        let request = Request::new_with_str_and_init(&initiate_url, &opts)?;
        request.headers().set("Content-Type", "application/json")?;
        
        let window = web_sys::window().ok_or_else(|| JsValue::from_str("No window"))?;
        let resp_value = JsFuture::from(window.fetch_with_request(&request)).await?;
        let resp: Response = resp_value.dyn_into()?;
        
        if !resp.ok() {
            return Err(JsValue::from_str(&format!(
                "Initiate failed: HTTP {}",
                resp.status()
            )));
        }
        
        let json = JsFuture::from(resp.json()?).await?;
        
        let init_resp: InitiateMessageResponse = serde_wasm_bindgen::from_value(json)
            .map_err(|e| JsValue::from_str(&format!("Failed to parse initiate response: {}", e)))?;
        
        // Phase 2: Upload or inline confirm
        if init_resp.should_upload {
            // Large message - upload to S3
            let upload_url = init_resp.upload_url.ok_or_else(|| 
                JsValue::from_str("Server indicated upload but no URL provided"))?;
            
            let opts2 = RequestInit::new();
            opts2.set_method("PUT");
            opts2.set_mode(RequestMode::Cors);
            
            let array = js_sys::Uint8Array::from(&message_bytes[..]);
            opts2.set_body(&array);
            
            let request2 = Request::new_with_str_and_init(&upload_url, &opts2)?;
            let resp_value2 = JsFuture::from(window.fetch_with_request(&request2)).await?;
            let resp2: Response = resp_value2.dyn_into()?;
            
            if !resp2.ok() {
                return Err(JsValue::from_str(&format!(
                    "S3 upload failed: HTTP {}",
                    resp2.status()
                )));
            }
            
            // Phase 3: Confirm S3 upload
            let confirm_url = format!(
                "{}/boards/{}/messages/{}/confirm",
                self.params.b4_url, board_name, init_resp.message_id
            );
            
            let confirm_req = ConfirmMessageRequest {
                data: None,
                version: http_message.version.clone(),
            };
            
            let confirm_json = serde_json::to_string(&confirm_req)
                .map_err(|e| JsValue::from_str(&format!("Failed to serialize confirm: {}", e)))?;
            
            let opts3 = RequestInit::new();
            opts3.set_method("POST");
            opts3.set_mode(RequestMode::Cors);
            opts3.set_body(&JsValue::from_str(&confirm_json));
            
            let request3 = Request::new_with_str_and_init(&confirm_url, &opts3)?;
            request3.headers().set("Content-Type", "application/json")?;
            
            let resp_value3 = JsFuture::from(window.fetch_with_request(&request3)).await?;
            let resp3: Response = resp_value3.dyn_into()?;
            
            if !resp3.ok() {
                return Err(JsValue::from_str(&format!(
                    "Confirm failed: HTTP {}",
                    resp3.status()
                )));
            }
        } else {
            // Small message - send inline
            let confirm_url = format!(
                "{}/boards/{}/messages/{}/confirm",
                self.params.b4_url, board_name, init_resp.message_id
            );
            
            let confirm_req = ConfirmMessageRequest {
                data: Some(message_bytes),
                version: http_message.version,
            };
            
            let confirm_json = serde_json::to_string(&confirm_req)
                .map_err(|e| JsValue::from_str(&format!("Failed to serialize confirm: {}", e)))?;
            
            let opts3 = RequestInit::new();
            opts3.set_method("POST");
            opts3.set_mode(RequestMode::Cors);
            opts3.set_body(&JsValue::from_str(&confirm_json));
            
            let request3 = Request::new_with_str_and_init(&confirm_url, &opts3)?;
            request3.headers().set("Content-Type", "application/json")?;
            
            let resp_value3 = JsFuture::from(window.fetch_with_request(&request3)).await?;
            let resp3: Response = resp_value3.dyn_into()?;
            
            if !resp3.ok() {
                return Err(JsValue::from_str(&format!(
                    "Confirm failed: HTTP {}",
                    resp3.status()
                )));
            }
        }
        
        Ok(())
    }
}

impl<C: Context> Board<C> for WasmHttpBoard {
    type Factory = WasmHttpBoardFactory;

    async fn get_messages(&mut self, board_name: &str, last_id: i64) -> anyhow::Result<Vec<HttpB4Message>> {
        self.fetch_messages_internal(board_name, last_id)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to fetch messages: {:?}", e))
    }

    async fn post_messages(&mut self, board_name: &str, messages: Vec<HttpB4Message>) -> anyhow::Result<()> {
        for http_message in messages {
            self.post_http_message_internal(board_name, http_message)
                .await
                .map_err(|e| anyhow::anyhow!("Failed to post message: {:?}", e))?;
        }
        Ok(())
    }
}

/// Factory for creating WasmHttpBoard instances
#[derive(Clone)]
pub struct WasmHttpBoardFactory {
    params: WasmHttpBoardParams,
}

impl WasmHttpBoardFactory {
    pub fn new(params: WasmHttpBoardParams) -> Self {
        WasmHttpBoardFactory { params }
    }
}

impl<C: Context> BoardFactory<C, WasmHttpBoard> for WasmHttpBoardFactory {
    fn get_board(&self) -> WasmHttpBoard {
        WasmHttpBoard::new(self.params.clone())
    }
}
