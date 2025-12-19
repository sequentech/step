// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

mod bulletin_board;
mod local_storage;
mod s3;
pub mod trustee;

use b4::api_types::Message;
use wasm_bindgen::prelude::*;

pub use bulletin_board::BulletinBoardClient;
pub use local_storage::LocalStorage;
pub use trustee::WasmTrustee;

/// Initialize the WASM module with console logging
#[wasm_bindgen(start)]
pub fn init() {
    console_error_panic_hook::set_once();

    // Set up tracing to output to browser console
    tracing_wasm::set_as_global_default();
}

// Re-export wasm-bindgen-rayon's initThreadPool for browser usage
// This provides parallel computation support via Web Workers
pub use wasm_bindgen_rayon::init_thread_pool as initThreadPool;

/// Main client that coordinates bulletin board, S3, and local storage
#[wasm_bindgen]
pub struct Client {
    bb_client: BulletinBoardClient,
    storage: LocalStorage,
}

#[wasm_bindgen]
impl Client {
    #[wasm_bindgen(constructor)]
    pub fn new(service_url: String) -> Result<Client, JsValue> {
        Ok(Client {
            bb_client: BulletinBoardClient::new(service_url.clone()),
            storage: LocalStorage::new("wbraid-cache")?,
        })
    }

    /// Post a new message to the bulletin board
    pub async fn post_message(&self, data: Vec<u8>) -> Result<JsValue, JsValue> {
        let size = data.len();

        // Phase 1: Initiate message upload
        let response = self
            .bb_client
            .initiate_message(size, "unknown".to_string(), "Unknown".to_string(), 0, 0)
            .await?;
        let message_id = response.message_id.clone();

        // Phase 2: Upload data
        if response.should_upload {
            // Large message - upload to S3
            if let Some(upload_url) = &response.upload_url {
                s3::upload_to_s3(upload_url, &data).await?;

                // Phase 3: Confirm S3 upload (no data in request)
                self.bb_client
                    .confirm_message(
                        &message_id,
                        None,
                        "unknown".to_string(),
                        "Unknown".to_string(),
                        0,
                        0,
                    )
                    .await?;
            }
        } else {
            // Small message - send inline data in confirm request
            self.bb_client
                .confirm_message(
                    &message_id,
                    Some(data.clone()),
                    "unknown".to_string(),
                    "Unknown".to_string(),
                    0,
                    0,
                )
                .await?;
        }

        // Cache locally
        self.storage.cache_message_data(&message_id, &data).await?;

        // Return a simple response object
        Ok(serde_wasm_bindgen::to_value(&serde_json::json!({
            "message_id": message_id,
        }))?)
    }

    /// Get a specific message by ID
    pub async fn get_message(&self, message_id: String) -> Result<JsValue, JsValue> {
        // Check local cache first
        if let Ok(Some(cached_data)) = self.storage.get_cached_message_data(&message_id).await {
            let message = Message {
                id: message_id.clone(),
                timestamp: 0, // Would need to store this
                size: cached_data.len(),
                content_type: b4::api_types::ContentType::Inline { data: cached_data },
                sender_pk: "unknown".to_string(),
                statement_kind: "Unknown".to_string(),
                batch: 0,
                mix_number: 0,
            };
            return Ok(serde_wasm_bindgen::to_value(&message)?);
        }

        // Fetch from bulletin board
        let response = self.bb_client.get_message(&message_id).await?;

        // If there's a download URL, fetch from S3
        let data = if let Some(download_url) = &response.download_url {
            s3::download_from_s3(download_url).await?
        } else {
            match &response.message.content_type {
                b4::api_types::ContentType::Inline { data } => data.clone(),
                _ => Vec::new(),
            }
        };

        // Cache locally
        self.storage.cache_message_data(&message_id, &data).await?;

        Ok(serde_wasm_bindgen::to_value(&response.message)?)
    }

    /// List all messages from the bulletin board
    pub async fn list_messages(&self) -> Result<JsValue, JsValue> {
        let response = self.bb_client.list_messages().await?;

        // Store metadata in local cache
        for message in &response.messages {
            self.storage.cache_message_metadata(message).await?;
        }

        Ok(serde_wasm_bindgen::to_value(&response.messages)?)
    }

    /// Get cached messages from local storage
    pub async fn get_cached_messages(&self) -> Result<JsValue, JsValue> {
        let messages = self.storage.get_all_cached_metadata().await?;
        Ok(serde_wasm_bindgen::to_value(&messages)?)
    }

    /// Clear local cache
    pub async fn clear_cache(&self) -> Result<(), JsValue> {
        self.storage.clear().await
    }
}

// Re-export console_error_panic_hook
use console_error_panic_hook;
