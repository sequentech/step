// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use b4::api_types::Message;
use indexed_db_futures::prelude::*;
use wasm_bindgen::prelude::*;

const DB_VERSION: u32 = 1;
const METADATA_STORE: &str = "message_metadata";
const DATA_STORE: &str = "message_data";

pub struct LocalStorage {
    db_name: String,
}

impl LocalStorage {
    pub fn new(db_name: &str) -> Result<Self, JsValue> {
        Ok(Self {
            db_name: db_name.to_string(),
        })
    }

    async fn open_db(&self) -> Result<IdbDatabase, JsValue> {
        let mut db_req = IdbDatabase::open_u32(&self.db_name, DB_VERSION)
            .map_err(|e| JsValue::from_str(&format!("Failed to open DB: {:?}", e)))?;

        db_req.set_on_upgrade_needed(Some(|evt: &IdbVersionChangeEvent| -> Result<(), JsValue> {
            // Create object stores if they don't exist
            if evt.db().name() == evt.db().name() {
                if !evt.db().object_store_names().any(|n| n == METADATA_STORE) {
                    evt.db().create_object_store(METADATA_STORE)?;
                }
                if !evt.db().object_store_names().any(|n| n == DATA_STORE) {
                    evt.db().create_object_store(DATA_STORE)?;
                }
            }
            Ok(())
        }));

        db_req
            .await
            .map_err(|e| JsValue::from_str(&format!("Failed to open DB: {:?}", e)))
    }

    /// Cache message metadata
    pub async fn cache_message_metadata(&self, message: &Message) -> Result<(), JsValue> {
        let db = self.open_db().await?;
        let tx = db
            .transaction_on_one_with_mode(METADATA_STORE, IdbTransactionMode::Readwrite)
            .map_err(|e| JsValue::from_str(&format!("Failed to create transaction: {:?}", e)))?;
        let store = tx
            .object_store(METADATA_STORE)
            .map_err(|e| JsValue::from_str(&format!("Failed to get object store: {:?}", e)))?;

        let json = serde_json::to_string(message)
            .map_err(|e| JsValue::from_str(&format!("Failed to serialize message: {}", e)))?;

        store
            .put_key_val(&JsValue::from_str(&message.id), &JsValue::from_str(&json))
            .map_err(|e| JsValue::from_str(&format!("Failed to put value: {:?}", e)))?;

        tx.await
            .into_result()
            .map_err(|e| JsValue::from_str(&format!("Transaction failed: {:?}", e)))?;

        Ok(())
    }

    /// Cache message data (blob)
    pub async fn cache_message_data(&self, message_id: &str, data: &[u8]) -> Result<(), JsValue> {
        let db = self.open_db().await?;
        let tx = db
            .transaction_on_one_with_mode(DATA_STORE, IdbTransactionMode::Readwrite)
            .map_err(|e| JsValue::from_str(&format!("Failed to create transaction: {:?}", e)))?;
        let store = tx
            .object_store(DATA_STORE)
            .map_err(|e| JsValue::from_str(&format!("Failed to get object store: {:?}", e)))?;

        let js_array = js_sys::Uint8Array::from(data);
        let js_value: JsValue = js_array.into();

        store
            .put_key_val(&JsValue::from_str(message_id), &js_value)
            .map_err(|e| JsValue::from_str(&format!("Failed to put value: {:?}", e)))?;

        tx.await
            .into_result()
            .map_err(|e| JsValue::from_str(&format!("Transaction failed: {:?}", e)))?;

        Ok(())
    }

    /// Get cached message data
    pub async fn get_cached_message_data(
        &self,
        message_id: &str,
    ) -> Result<Option<Vec<u8>>, JsValue> {
        let db = self.open_db().await?;
        let tx = db
            .transaction_on_one(DATA_STORE)
            .map_err(|e| JsValue::from_str(&format!("Failed to create transaction: {:?}", e)))?;
        let store = tx
            .object_store(DATA_STORE)
            .map_err(|e| JsValue::from_str(&format!("Failed to get object store: {:?}", e)))?;

        let value = store
            .get(&JsValue::from_str(message_id))
            .map_err(|e| JsValue::from_str(&format!("Failed to get value: {:?}", e)))?
            .await
            .map_err(|e| JsValue::from_str(&format!("Failed to retrieve value: {:?}", e)))?;

        match value {
            None => return Ok(None),
            Some(js_val) if js_val.is_undefined() || js_val.is_null() => return Ok(None),
            Some(js_val) => {
                let array = js_sys::Uint8Array::new(&js_val);
                Ok(Some(array.to_vec()))
            }
        }
    }

    /// Get all cached message metadata
    pub async fn get_all_cached_metadata(&self) -> Result<Vec<Message>, JsValue> {
        let db = self.open_db().await?;
        let tx = db
            .transaction_on_one(METADATA_STORE)
            .map_err(|e| JsValue::from_str(&format!("Failed to create transaction: {:?}", e)))?;
        let store = tx
            .object_store(METADATA_STORE)
            .map_err(|e| JsValue::from_str(&format!("Failed to get object store: {:?}", e)))?;

        let mut messages = Vec::new();
        let cursor = store
            .open_cursor()
            .map_err(|e| JsValue::from_str(&format!("Failed to open cursor: {:?}", e)))?
            .await
            .map_err(|e| JsValue::from_str(&format!("Failed to get cursor: {:?}", e)))?;

        if let Some(cursor) = cursor {
            loop {
                let value = cursor.value();
                if let Some(json) = value.as_string() {
                    if let Ok(message) = serde_json::from_str::<Message>(&json) {
                        messages.push(message);
                    }
                }

                if !cursor
                    .continue_cursor()
                    .map_err(|e| JsValue::from_str(&format!("Failed to continue cursor: {:?}", e)))?
                    .await
                    .map_err(|e| JsValue::from_str(&format!("Failed to advance cursor: {:?}", e)))?
                {
                    break;
                }
            }
        }

        Ok(messages)
    }

    /// Clear all cached data
    pub async fn clear(&self) -> Result<(), JsValue> {
        let db = self.open_db().await?;

        // Clear metadata
        let tx = db
            .transaction_on_one_with_mode(METADATA_STORE, IdbTransactionMode::Readwrite)
            .map_err(|e| JsValue::from_str(&format!("Failed to create transaction: {:?}", e)))?;
        let store = tx
            .object_store(METADATA_STORE)
            .map_err(|e| JsValue::from_str(&format!("Failed to get object store: {:?}", e)))?;
        store
            .clear()
            .map_err(|e| JsValue::from_str(&format!("Failed to clear store: {:?}", e)))?;
        tx.await
            .into_result()
            .map_err(|e| JsValue::from_str(&format!("Transaction failed: {:?}", e)))?;

        // Clear data
        let tx = db
            .transaction_on_one_with_mode(DATA_STORE, IdbTransactionMode::Readwrite)
            .map_err(|e| JsValue::from_str(&format!("Failed to create transaction: {:?}", e)))?;
        let store = tx
            .object_store(DATA_STORE)
            .map_err(|e| JsValue::from_str(&format!("Failed to get object store: {:?}", e)))?;
        store
            .clear()
            .map_err(|e| JsValue::from_str(&format!("Failed to clear store: {:?}", e)))?;
        tx.await
            .into_result()
            .map_err(|e| JsValue::from_str(&format!("Transaction failed: {:?}", e)))?;

        Ok(())
    }
}
