// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! WASM bindings for Braid mixnet node and session

use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::{Request, RequestInit, RequestMode, Response};

use crate::protocol::board::local_storage::LocalBoardStorage;
use crate::protocol::board::BoardEntry;
use crate::protocol::session::Session;
use crate::protocol::trustee::{Trustee, TrusteeConfig};
use crate::wasm::board::{IndexedDbStorage, WasmHttpBoardFactory, WasmHttpBoardParams};
use b4::api_types::{
    ConfirmMessageRequest, ContentType, InitiateMessageRequest, InitiateMessageResponse,
    ListMessagesResponse,
};
use b4::HttpB3Message;
use strand::backend::ristretto::RistrettoCtx;
use strand::signature::StrandSignatureSk;
use strand::symm;

/// WASM-specific configuration that includes session properties
/// This wraps the core TrusteeConfig with additional WASM UI needs
#[derive(Serialize, Deserialize)]
pub struct WasmSessionConfig {
    // Trustee instance name
    // FIXME is this used anywhere?
    pub name: String,
    pub b4_url: String,
    #[serde(flatten)]
    pub trustee_config: TrusteeConfig,
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
pub struct SessionState {
    pub board_name: String,
    pub current_messages: usize,
    pub max_messages: usize,
    pub last_message_id: i64,
}

/// Main WASM session interface
///
/// Wraps a braid::protocol::session::Session with browser-specific functionality
/// The protocol execution cycle is managed by the inner session.
#[wasm_bindgen]
pub struct WasmSession {
    session: Option<Session<RistrettoCtx, crate::wasm::board::WasmHttpBoard, IndexedDbStorage>>,
    // Trustee instance name
    // FIXME is this used anywhere?
    name: String,
    b4_url: String,
    board_name: Option<String>,
    config: TrusteeConfig,
}

#[wasm_bindgen]
impl WasmSession {
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
    pub fn new(config_json: String) -> Result<WasmSession, JsValue> {
        console_error_panic_hook::set_once();

        let wasm_config: WasmSessionConfig = serde_json::from_str(&config_json)
            .map_err(|e| JsValue::from_str(&format!("Failed to parse config: {}", e)))?;

        Ok(WasmSession {
            session: None,
            name: wasm_config.name,
            b4_url: wasm_config.b4_url,
            board_name: None,
            config: wasm_config.trustee_config,
        })
    }

    /// Initialize a session for a specific board
    ///
    /// This creates the Trustee object and initializes IndexedDB storage.
    /// Must be called before connect_to_board() or step().
    pub async fn init_session(&mut self, board_name: String) -> Result<(), JsValue> {
        // Parse signing key
        let sk = StrandSignatureSk::from_der_b64_string(&self.config.signing_key_sk)
            .map_err(|e| JsValue::from_str(&format!("Failed to parse signing key: {}", e)))?;

        // Parse encryption key
        let bytes = crate::util::decode_base64(&self.config.encryption_key)
            .map_err(|e| JsValue::from_str(&format!("Failed to decode encryption key: {}", e)))?;
        let ek = symm::sk_from_bytes(&bytes)
            .map_err(|e| JsValue::from_str(&format!("Failed to parse encryption key: {}", e)))?;

        // Create IndexedDB storage (persistent, tamper-resistant)
        let storage = IndexedDbStorage::new(format!("braid_{}", board_name));

        // Initialize storage (open IndexedDB and load metadata)
        storage
            .init()
            .await
            .map_err(|e| JsValue::from_str(&format!("Failed to initialize storage: {}", e)))?;

        web_sys::console::log_1(&JsValue::from_str(
            "Using IndexedDbStorage (persistent, metadata-only)",
        ));

        let trustee = Trustee::new(
            self.name.clone(),
            board_name.clone(),
            sk,
            ek,
            storage,
            None, // Default max_concurrent_actions
        );

        // Create board factory
        let board_factory = WasmHttpBoardFactory::new(WasmHttpBoardParams {
            b4_url: self.b4_url.clone(),
        });

        // Create session
        let session = Session::new(&board_name, trustee, board_factory);

        self.session = Some(session);
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
        let session = self.session.as_mut().ok_or_else(|| {
            JsValue::from_str("Session not initialized. Call init_session() first")
        })?;

        web_sys::console::log_1(&JsValue::from_str(
            "Connecting to board and checking for pending messages...",
        ));

        // SECURITY CRITICAL: Get last external ID from storage (not from memory)
        // This ensures we resume from persisted state, not transient state
        let last_id = session
            .trustee
            .get_last_external_id()
            .map_err(|e| JsValue::from_str(&format!("Failed to get last ID: {:?}", e)))?;

        web_sys::console::log_1(&JsValue::from_str(&format!(
            "Last stored external ID: {}",
            last_id
        )));

        // Fetch messages from bulletin board to see what's available
        // NOTE: We only fetch here, we don't store or process yet
        // Actual storage and processing happens during protocol steps
        let messages = self.fetch_messages(last_id).await?;

        let pending_count = messages.len();

        if pending_count > 0 {
            web_sys::console::log_1(&JsValue::from_str(&format!(
                "✓ Connected to board: {} remote messages pending (will be processed in protocol steps)",
                pending_count
            )));
        } else {
            web_sys::console::log_1(&JsValue::from_str("✓ Connected to board: no new messages"));
        }

        #[derive(Serialize)]
        struct ConnectInfo {
            pending: usize,
            last_external_id: i64,
        }

        serde_wasm_bindgen::to_value(&ConnectInfo {
            pending: pending_count,
            last_external_id: last_id,
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
    /// Returns information about actions taken
    pub async fn step(&mut self) -> Result<JsValue, JsValue> {
        let session = self.session.as_mut().ok_or_else(|| {
            JsValue::from_str("Session not initialized. Call init_session() first")
        })?;

        // Use the generic Session::step() method which returns (posted_count, StepResult)
        let (posted_count, step_result) = session.step().await.map_err(|e| {
            let error_msg = format!("Step failed: {:?}", e);
            web_sys::console::error_1(&JsValue::from_str(&error_msg));
            JsValue::from_str(&error_msg)
        })?;

        // Persist metadata to IndexedDB after protocol step
        session
            .trustee
            .local_board
            .storage
            .persist()
            .await
            .map_err(|e| {
                let error_msg = format!("Failed to persist storage: {:?}", e);
                web_sys::console::error_1(&JsValue::from_str(&error_msg));
                JsValue::from_str(&error_msg)
            })?;

        #[derive(Serialize)]
        struct StepInfo {
            added: i64,
            posted: usize,
            actions: Vec<String>,
        }

        // Convert actions to strings for display (just variant names)
        let action_strings: Vec<String> = step_result
            .actions
            .iter()
            .map(|a| {
                // Extract just the variant name from Debug format
                // e.g., "SignConfiguration(...)" -> "SignConfiguration"
                let debug_str = format!("{:?}", a);
                debug_str
                    .split('(')
                    .next()
                    .unwrap_or(&debug_str)
                    .to_string()
            })
            .collect();

        serde_wasm_bindgen::to_value(&StepInfo {
            added: step_result.added_messages,
            posted: posted_count,
            actions: action_strings,
        })
        .map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e)))
    }

    /// Get current state of the trustee for visualization
    pub fn get_state(&self) -> Result<JsValue, JsValue> {
        let session = self
            .session
            .as_ref()
            .ok_or_else(|| JsValue::from_str("Session not initialized"))?;

        let board_name = self
            .board_name
            .as_ref()
            .ok_or_else(|| JsValue::from_str("Session not initialized"))?;

        let config = if session.trustee.local_board.configuration.is_some() {
            1
        } else {
            0
        };

        // Access trustee fields directly
        let state = SessionState {
            board_name: board_name.clone(),
            current_messages: session.trustee.local_board.get_statement_entries().len() + config,
            max_messages: session.trustee.local_board.max_messages(),
            last_message_id: session.trustee.local_board.get_last_local_board_id(),
        };

        serde_wasm_bindgen::to_value(&state)
            .map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e)))
    }

    /// Clear persistent storage (for testing)
    ///
    /// This clears the IndexedDB storage, allowing a fresh start.
    /// Note: This does NOT reset the session - you'll need to reload the page
    /// or call init_session() again after clearing.
    pub async fn clear_storage(&self) -> Result<(), JsValue> {
        let session = self
            .session
            .as_ref()
            .ok_or_else(|| JsValue::from_str("Session not initialized"))?;

        session
            .trustee
            .local_board
            .storage
            .clear()
            .await
            .map_err(|e| {
                let error_msg = format!("Failed to clear storage: {:?}", e);
                web_sys::console::error_1(&JsValue::from_str(&error_msg));
                JsValue::from_str(&error_msg)
            })
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
        let session = self
            .session
            .as_ref()
            .ok_or_else(|| JsValue::from_str("Session not initialized"))?;

        #[derive(Serialize)]
        struct StatementInfo {
            kind: String,
            signer: usize,
            batch: u64,
            mix: usize,
        }

        let entries: Vec<BoardEntry> = session.trustee.local_board.get_statement_entries();
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

    /// Get storage diagnostics information
    ///
    /// Returns information about the storage backend including:
    /// - Backend type (IndexedDbStorage)
    /// - Total messages stored (hash metadata count)
    /// - Maximum internal ID (locally-controlled, security-critical)
    /// - Maximum external ID (from bulletin board, optimization only)
    pub fn get_storage_info(&self) -> Result<JsValue, JsValue> {
        let session = self
            .session
            .as_ref()
            .ok_or_else(|| JsValue::from_str("Session not initialized"))?;

        let info = session
            .trustee
            .local_board
            .storage
            .get_storage_info()
            .map_err(|e| JsValue::from_str(&format!("Failed to get storage info: {}", e)))?;

        serde_wasm_bindgen::to_value(&info)
            .map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e)))
    }
}
