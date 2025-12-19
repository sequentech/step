// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! WASM bindings for the Braid mixnet trustee

use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::{Request, RequestInit, RequestMode, Response};

use b4::api_types::{
    ConfirmMessageRequest, ContentType, InitiateMessageRequest, InitiateMessageResponse,
    ListMessagesResponse,
};
use b4::HttpB3Message;
use braid::protocol::board::local::LocalBoardStorage;
use braid::protocol::trustee::{Trustee, TrusteeConfig};
use strand::backend::ristretto::RistrettoCtx;
use strand::signature::StrandSignatureSk;
use strand::symm;

/// WASM-specific configuration that includes session properties
/// This wraps the core TrusteeConfig with additional WASM UI needs
#[derive(Serialize, Deserialize)]
pub struct WasmTrusteeConfig {
    pub name: String,   // Trustee instance name (not in core config)
    pub b4_url: String, // HTTP endpoint for this session (not trustee property)
    #[serde(flatten)]
    pub trustee_config: TrusteeConfig, // Core cryptographic config from braid
}

/// Board information for UI display
/// Note: B4's BoardResponse has additional fields (created_at, status) that we don't need here.
/// This simplified version is sufficient for the WASM UI's board selection.
#[derive(Serialize, Deserialize)]
pub struct BoardInfo {
    pub name: String,
}

/// State information about the trustee's progress
#[derive(Serialize, Deserialize)]
pub struct TrusteeState {
    pub board_name: String,
    pub current_messages: usize,
    pub max_messages: usize,
    pub last_message_id: i64,
}

/// Main WASM trustee interface
#[wasm_bindgen]
pub struct WasmTrustee {
    trustee: Option<Trustee<RistrettoCtx>>,
    name: String,               // Session: Trustee instance name
    b4_url: String,             // Session: HTTP endpoint
    board_name: Option<String>, // Session: Current board
    config: TrusteeConfig,      // Core: Cryptographic configuration
}

#[wasm_bindgen]
impl WasmTrustee {
    /// Create a new trustee from a JSON configuration string
    ///
    /// Config format:
    /// ```json
    /// {
    ///   "name": "trustee1",
    ///   "b4_url": "http://localhost:8000",
    ///   "signing_key_sk": "<base64-der>",
    ///   "signing_key_pk": "<base64-der>",
    ///   "encryption_key": "<base64>"
    /// }
    /// ```
    #[wasm_bindgen(constructor)]
    pub fn new(config_json: String) -> Result<WasmTrustee, JsValue> {
        console_error_panic_hook::set_once();

        let wasm_config: WasmTrusteeConfig = serde_json::from_str(&config_json)
            .map_err(|e| JsValue::from_str(&format!("Failed to parse config: {}", e)))?;

        Ok(WasmTrustee {
            trustee: None,
            name: wasm_config.name,
            b4_url: wasm_config.b4_url,
            board_name: None,
            config: wasm_config.trustee_config,
        })
    }

    /// Initialize a session for a specific board
    ///
    /// This creates the Trustee object needed to process messages
    pub fn init_session(&mut self, board_name: String) -> Result<(), JsValue> {
        // Parse signing key
        let sk = StrandSignatureSk::from_der_b64_string(&self.config.signing_key_sk)
            .map_err(|e| JsValue::from_str(&format!("Failed to parse signing key: {}", e)))?;

        // Parse encryption key
        let bytes = braid::util::decode_base64(&self.config.encryption_key)
            .map_err(|e| JsValue::from_str(&format!("Failed to decode encryption key: {}", e)))?;
        let ek = symm::sk_from_bytes(&bytes)
            .map_err(|e| JsValue::from_str(&format!("Failed to parse encryption key: {}", e)))?;

        // Create trustee (no persistent store for WASM - in-memory only)
        let trustee = Trustee::new(
            self.name.clone(),
            board_name.clone(),
            sk,
            ek,
            None, // No store for WASM
            None, // Default max_concurrent_actions
        );

        self.trustee = Some(trustee);
        self.board_name = Some(board_name.clone());

        web_sys::console::log_1(&JsValue::from_str(&format!(
            "Initialized session for board '{}'",
            board_name
        )));

        Ok(())
    }

    /// Connect to a board and perform initial synchronization
    ///
    /// This should be called after init_session() to fetch existing messages
    /// from the remote board and update the local board state, without executing
    /// any protocol steps. This allows the UI to display the current board state
    /// before the user starts stepping through the protocol.
    ///
    /// Returns the number of messages fetched and added to the local board.
    pub async fn connect_to_board(&mut self) -> Result<JsValue, JsValue> {
        // Ensure we have a session
        let trustee = self.trustee.as_mut().ok_or_else(|| {
            JsValue::from_str("Session not initialized. Call init_session() first")
        })?;

        web_sys::console::log_1(&JsValue::from_str(
            "Connecting to board and fetching existing messages...",
        ));

        // Get last external ID (should be 0 for a fresh connection)
        let last_id = trustee
            .get_last_external_id()
            .map_err(|e| JsValue::from_str(&format!("Failed to get last ID: {:?}", e)))?;

        web_sys::console::log_1(&JsValue::from_str(&format!(
            "Fetching messages after ID {}",
            last_id
        )));

        // Fetch messages from B4
        let messages = self.fetch_messages(last_id).await?;

        web_sys::console::log_1(&JsValue::from_str(&format!(
            "Received {} messages from board",
            messages.len()
        )));

        // Update the local board without executing actions
        let (added_messages, last_message_id) = {
            let trustee = self
                .trustee
                .as_mut()
                .ok_or_else(|| JsValue::from_str("Trustee disappeared"))?;

            // Convert HttpB3Message to (Message, i64) pairs
            let parsed_messages: Result<Vec<_>, JsValue> = messages
                .iter()
                .map(|m| {
                    use b4::messages::message::Message;
                    use strand::serialization::StrandDeserialize;

                    let message = Message::strand_deserialize(&m.message).map_err(|e| {
                        JsValue::from_str(&format!("Failed to deserialize message: {:?}", e))
                    })?;
                    Ok((message, m.id))
                })
                .collect();

            let parsed_messages = parsed_messages?;

            // Update local board (this is what step() does internally)
            trustee
                .update_local_board(parsed_messages)
                .map_err(|e| JsValue::from_str(&format!("Failed to update local board: {:?}", e)))?
        };

        // Update the last_local_board_id in trustee
        if added_messages > 0 {
            let trustee = self
                .trustee
                .as_mut()
                .ok_or_else(|| JsValue::from_str("Trustee disappeared"))?;
            trustee.last_local_board_id = last_message_id;

            web_sys::console::log_1(&JsValue::from_str(&format!(
                "✓ Connected to board: {} messages added, last_local_board_id = {}",
                added_messages, last_message_id
            )));
        } else {
            web_sys::console::log_1(&JsValue::from_str("✓ Connected to board: no new messages"));
        }

        #[derive(Serialize)]
        struct ConnectInfo {
            added: i64,
            last_message_id: i64,
        }

        serde_wasm_bindgen::to_value(&ConnectInfo {
            added: added_messages,
            last_message_id,
        })
        .map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e)))
    }

    /// Fetch list of available boards from B4
    pub async fn fetch_boards(&self) -> Result<JsValue, JsValue> {
        let url = format!("{}/boards", self.b4_url);

        let opts = RequestInit::new();
        opts.set_method("GET");
        opts.set_mode(RequestMode::Cors);

        let request = Request::new_with_str_and_init(&url, &opts)?;

        let window = web_sys::window().ok_or_else(|| JsValue::from_str("No window"))?;
        let resp_value = JsFuture::from(window.fetch_with_request(&request)).await?;
        let resp: Response = resp_value.dyn_into()?;

        if !resp.ok() {
            return Err(JsValue::from_str(&format!("HTTP error: {}", resp.status())));
        }

        let json = JsFuture::from(resp.json()?).await?;

        // Parse the response which should be {"boards": [{"name": "board1", ...}, ...]}
        let boards_obj = js_sys::Reflect::get(&json, &JsValue::from_str("boards"))?;
        let boards_array: js_sys::Array = boards_obj.dyn_into()?;

        let board_infos: Vec<BoardInfo> = boards_array
            .iter()
            .filter_map(|board_obj| {
                // Each board is an object with "name" field
                let name = js_sys::Reflect::get(&board_obj, &JsValue::from_str("name")).ok()?;
                let name_str = name.as_string()?;
                Some(BoardInfo { name: name_str })
            })
            .collect();

        web_sys::console::log_1(&JsValue::from_str(&format!(
            "Fetched {} boards",
            board_infos.len()
        )));

        serde_wasm_bindgen::to_value(&board_infos)
            .map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e)))
    }

    /// Fetch messages from B4 for the current board
    async fn fetch_messages(&self, last_id: i64) -> Result<Vec<HttpB3Message>, JsValue> {
        let board_name = self
            .board_name
            .as_ref()
            .ok_or_else(|| JsValue::from_str("Session not initialized"))?;

        let url = format!(
            "{}/boards/{}/messages?last_id={}",
            self.b4_url, board_name, last_id
        );

        let opts = RequestInit::new();
        opts.set_method("GET");
        opts.set_mode(RequestMode::Cors);

        let request = Request::new_with_str_and_init(&url, &opts)?;

        let window = web_sys::window().ok_or_else(|| JsValue::from_str("No window"))?;
        let resp_value = JsFuture::from(window.fetch_with_request(&request)).await?;
        let resp: Response = resp_value.dyn_into()?;

        if !resp.ok() {
            return Err(JsValue::from_str(&format!("HTTP error: {}", resp.status())));
        }

        let json = JsFuture::from(resp.json()?).await?;

        // Parse using wbraid-shared types
        let list_response: ListMessagesResponse = serde_wasm_bindgen::from_value(json)
            .map_err(|e| JsValue::from_str(&format!("Failed to parse messages response: {}", e)))?;

        web_sys::console::log_1(&JsValue::from_str(&format!(
            "✓ Parsed {} messages from response",
            list_response.messages.len()
        )));

        // Convert to HttpB3Message, fetching S3 content when needed
        let mut messages = Vec::new();

        for http_msg in list_response.messages {
            let message_bytes = match http_msg.content_type {
                ContentType::Inline { data } => data,
                ContentType::S3 { key } => {
                    web_sys::console::log_1(&JsValue::from_str(&format!(
                        "Message {} uses S3 storage ({}), fetching download URL...",
                        http_msg.id, key
                    )));

                    // Fetch download URL from B4's single-message endpoint
                    let message_url = format!(
                        "{}/boards/{}/messages/{}",
                        self.b4_url, board_name, http_msg.id
                    );

                    let opts = RequestInit::new();
                    opts.set_method("GET");
                    opts.set_mode(RequestMode::Cors);

                    let request = Request::new_with_str_and_init(&message_url, &opts)?;
                    let window = web_sys::window().ok_or_else(|| JsValue::from_str("No window"))?;
                    let resp_value = JsFuture::from(window.fetch_with_request(&request)).await?;
                    let resp: Response = resp_value.dyn_into()?;

                    if !resp.ok() {
                        return Err(JsValue::from_str(&format!(
                            "Failed to fetch message {}: HTTP {}",
                            http_msg.id,
                            resp.status()
                        )));
                    }

                    let json = JsFuture::from(resp.json()?).await?;
                    let download_url =
                        js_sys::Reflect::get(&json, &JsValue::from_str("download_url"))
                            .map_err(|e| {
                                JsValue::from_str(&format!("Failed to get download_url: {:?}", e))
                            })?
                            .as_string()
                            .ok_or_else(|| JsValue::from_str("download_url is not a string"))?;

                    web_sys::console::log_1(&JsValue::from_str(&format!(
                        "Downloading S3 binary content from: {}",
                        download_url
                    )));

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

                    // S3 stores raw binary data
                    let array_buffer = JsFuture::from(resp2.array_buffer()?).await?;
                    let uint8_array = js_sys::Uint8Array::new(&array_buffer);
                    let bytes = uint8_array.to_vec();

                    web_sys::console::log_1(&JsValue::from_str(&format!(
                        "Downloaded {} bytes from S3",
                        bytes.len()
                    )));

                    bytes
                }
            };

            let id: i64 = http_msg.id.parse().map_err(|e| {
                JsValue::from_str(&format!(
                    "Failed to parse message ID '{}': {}",
                    http_msg.id, e
                ))
            })?;

            messages.push(HttpB3Message::new(
                id,
                message_bytes,
                "1".to_string(),
                http_msg.sender_pk,
                http_msg.statement_kind,
                http_msg.batch,
                http_msg.mix_number,
            ));
        }

        web_sys::console::log_1(&JsValue::from_str(&format!(
            "Fetched {} messages",
            messages.len()
        )));

        Ok(messages)
    }

    /// Post messages to B4
    async fn post_messages(
        &self,
        messages: Vec<b4::messages::message::Message>,
    ) -> Result<(), JsValue> {
        use strand::serialization::StrandSerialize;

        if messages.is_empty() {
            return Ok(());
        }

        let board_name = self
            .board_name
            .as_ref()
            .ok_or_else(|| JsValue::from_str("Session not initialized"))?;

        web_sys::console::log_1(&JsValue::from_str(&format!(
            "Posting {} messages to board '{}'",
            messages.len(),
            board_name
        )));

        let num_messages = messages.len();

        for message in messages {
            // Extract metadata
            let sender_pk =
                message.sender.pk.to_der_b64_string().map_err(|e| {
                    JsValue::from_str(&format!("Failed to encode sender PK: {:?}", e))
                })?;
            let statement_kind = message.statement.get_kind().to_string();
            let batch: i32 = message.statement.get_batch_number() as i32;
            let mix_number: i32 = message.statement.get_mix_number() as i32;

            // Serialize message
            let message_bytes = message
                .strand_serialize()
                .map_err(|e| JsValue::from_str(&format!("Failed to serialize message: {:?}", e)))?;
            let size = message_bytes.len();

            web_sys::console::log_1(&JsValue::from_str(&format!(
                "Posting {} message (size: {} bytes)",
                statement_kind, size
            )));

            // Phase 1: Initiate message
            let initiate_url = format!("{}/boards/{}/messages/initiate", self.b4_url, board_name);

            let initiate_req = InitiateMessageRequest {
                size,
                sender_pk: sender_pk.clone(),
                statement_kind: statement_kind.clone(),
                batch,
                mix_number,
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

            let init_resp: InitiateMessageResponse =
                serde_wasm_bindgen::from_value(json).map_err(|e| {
                    JsValue::from_str(&format!("Failed to parse initiate response: {}", e))
                })?;

            web_sys::console::log_1(&JsValue::from_str(&format!(
                "Initiated message ID: {}, should_upload: {}",
                init_resp.message_id, init_resp.should_upload
            )));

            // Phase 2: Upload or inline confirm
            if init_resp.should_upload {
                // Large message - upload to S3
                let upload_url = init_resp.upload_url.ok_or_else(|| {
                    JsValue::from_str("Server indicated upload but no URL provided")
                })?;

                web_sys::console::log_1(&JsValue::from_str(&format!(
                    "Uploading {} bytes to S3...",
                    message_bytes.len()
                )));

                let opts2 = RequestInit::new();
                opts2.set_method("PUT");
                opts2.set_mode(RequestMode::Cors);

                // Convert Vec<u8> to Uint8Array for upload
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
                    self.b4_url, board_name, init_resp.message_id
                );

                let confirm_req = ConfirmMessageRequest {
                    data: None,
                    sender_pk,
                    statement_kind,
                    batch,
                    mix_number,
                };

                let confirm_json = serde_json::to_string(&confirm_req).map_err(|e| {
                    JsValue::from_str(&format!("Failed to serialize confirm: {}", e))
                })?;

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

                web_sys::console::log_1(&JsValue::from_str("✓ S3 message confirmed"));
            } else {
                // Small message - send inline
                let confirm_url = format!(
                    "{}/boards/{}/messages/{}/confirm",
                    self.b4_url, board_name, init_resp.message_id
                );

                let confirm_req = ConfirmMessageRequest {
                    data: Some(message_bytes),
                    sender_pk,
                    statement_kind,
                    batch,
                    mix_number,
                };

                let confirm_json = serde_json::to_string(&confirm_req).map_err(|e| {
                    JsValue::from_str(&format!("Failed to serialize confirm: {}", e))
                })?;

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

                web_sys::console::log_1(&JsValue::from_str("✓ Inline message confirmed"));
            }
        }

        web_sys::console::log_1(&JsValue::from_str(&format!(
            "Successfully posted all {} messages",
            num_messages
        )));

        Ok(())
    }

    /// Perform one protocol step
    ///
    /// This:
    /// 1. Fetches new messages from B4
    /// 2. Processes them through the trustee
    /// 3. Posts any resulting messages back to B4
    ///
    /// Returns the number of messages posted and messages added
    pub async fn step(&mut self) -> Result<JsValue, JsValue> {
        // Get last external ID first
        let last_id = {
            let trustee = self.trustee.as_mut().ok_or_else(|| {
                JsValue::from_str("Session not initialized. Call init_session() first")
            })?;

            trustee
                .get_last_external_id()
                .map_err(|e| JsValue::from_str(&format!("Failed to get last ID: {:?}", e)))?
        };

        web_sys::console::log_1(&JsValue::from_str(&format!(
            "Fetching messages after ID {}",
            last_id
        )));

        // Fetch messages from B4
        let messages = self.fetch_messages(last_id).await?;

        web_sys::console::log_1(&JsValue::from_str(&format!(
            "Received {} messages",
            messages.len()
        )));

        // Process through trustee
        let (num_posted, num_added, messages_to_post, action_strings) = {
            let trustee = self
                .trustee
                .as_mut()
                .ok_or_else(|| JsValue::from_str("Trustee disappeared"))?;

            let step_result = trustee.step(&messages).map_err(|e| {
                let error_msg = format!("Step failed: {:?}", e);
                web_sys::console::error_1(&JsValue::from_str(&error_msg));
                JsValue::from_str(&error_msg)
            })?;

            // Convert actions to strings using Display trait
            let actions: Vec<String> = step_result
                .actions
                .iter()
                .map(|a| format!("{}", a))
                .collect();

            (
                step_result.messages.len(),
                step_result.added_messages as usize,
                step_result.messages,
                actions,
            )
        };

        if num_posted > 0 {
            web_sys::console::log_1(&JsValue::from_str(&format!(
                "Posting {} messages",
                num_posted
            )));

            // Post messages back to B4
            self.post_messages(messages_to_post).await?;
        }

        #[derive(Serialize)]
        struct StepInfo {
            posted: usize,
            added: usize,
            actions: Vec<String>,
        }

        serde_wasm_bindgen::to_value(&StepInfo {
            posted: num_posted,
            added: num_added,
            actions: action_strings,
        })
        .map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e)))
    }

    /// Get current state of the trustee for visualization
    pub fn get_state(&self) -> Result<JsValue, JsValue> {
        let trustee = self
            .trustee
            .as_ref()
            .ok_or_else(|| JsValue::from_str("Session not initialized"))?;

        let board_name = self
            .board_name
            .as_ref()
            .ok_or_else(|| JsValue::from_str("Session not initialized"))?;

        // Access trustee fields directly
        let state = TrusteeState {
            board_name: board_name.clone(),
            current_messages: trustee.local_board.get_statement_entries().len() + 1, // +1 for config
            max_messages: trustee.local_board.max_messages(),
            last_message_id: trustee.last_local_board_id,
        };

        serde_wasm_bindgen::to_value(&state)
            .map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e)))
    }

    /// Get the current board name (if session initialized)
    #[wasm_bindgen(getter)]
    pub fn board_name(&self) -> Option<String> {
        self.board_name.clone()
    }

    /// Get the trustee name
    #[wasm_bindgen(getter)]
    pub fn name(&self) -> String {
        self.name.clone()
    }

    /// Get the B4 URL
    #[wasm_bindgen(getter)]
    pub fn b4_url(&self) -> String {
        self.b4_url.clone()
    }

    /// Get board summary - list of statements in local board
    pub fn get_board_summary(&self) -> Result<JsValue, JsValue> {
        use braid::protocol::board::local::BoardEntry;

        let trustee = self
            .trustee
            .as_ref()
            .ok_or_else(|| JsValue::from_str("Session not initialized"))?;

        #[derive(Serialize)]
        struct StatementInfo {
            kind: String,
            signer: usize,
            batch: u64,
            mix: usize,
        }

        let entries: Vec<BoardEntry> = trustee.local_board.get_statement_entries();
        let summary: Vec<StatementInfo> = entries
            .iter()
            .map(|e| StatementInfo {
                kind: e.key.kind.to_string(),
                signer: e.key.signer_position,
                batch: e.key.batch,
                mix: e.key.mix_number,
            })
            .collect();

        serde_wasm_bindgen::to_value(&summary)
            .map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e)))
    }
}
