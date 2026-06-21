// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! IndexedDB-backed storage for browser trustees
//!
//! This implementation provides persistent, tamper-resistant storage using a
//! metadata-only approach that stores message hashes and metadata in IndexedDB.
//!
//! # Security Model
//!
//! Instead of storing full messages (like SQLite), this implementation stores:
//! - **Hash list**: Ordered list of message hashes (append-only, insertion order)
//! - **Metadata set**: Message identifiers for duplicate detection
//!
//! This provides the same security properties as SQLite:
//! 1. **Append-only**: Hash list grows monotonically, tamper detection via hash verification
//! 2. **Tamper-resistant**: Any modification to historical messages detected by hash mismatch
//! 3. **Locally-controlled ordering**: Hash list position is equivalent to AUTOINCREMENT ID
//!
//! # Storage Strategy
//!
//! **Persistent (IndexedDB):**
//! - `hash_list: Vec<[u8; 32]>` - Ordered hashes for tamper detection
//! - `metadata_set: HashSet<MessageMetadata>` - Duplicate prevention
//!
//! **Transient (in-memory, NOT persisted):**
//! - `last_external_id: i64` - Optimization for fetching new messages within session
//! - `message_buffer: Vec<HttpB3Message>` - Messages between store/retrieve calls
//!
//! # Why No Persistent last_external_id?
//!
//! Unlike native SQLite which stores full messages and can reconstruct LocalBoard
//! from disk, this implementation only stores metadata. On session restart, all
//! messages must be re-fetched from the bulletin board and verified against stored
//! hashes. Therefore `last_external_id` only optimizes fetching within a session.
//!
//! # Verification Algorithm
//!
//! Given:
//! - S = Total hashes in metadata store
//! - B = Messages already in LocalBoard (trustee.last_local_board_id)
//! - Messages returned from bulletin board
//!
//! Verification rule:
//! ```
//! if B > S:
//!     ERROR: Corruption (LocalBoard ahead of metadata)
//!
//! verify_count = S - B
//! for i in 0..verify_count:
//!     if hash(messages[i]) != hash_list[B + i]:
//!         ERROR: Tamper detected!
//!
//! for msg in messages[verify_count..]:
//!     if metadata_set.contains(msg.metadata):
//!         if ignore_existing: skip
//!         else: ERROR: Duplicate
//!     hash_list.push(hash(msg))
//!     metadata_set.insert(msg.metadata)
//! ```
//!
//! This handles:
//! - Fresh restart (B=0): Verify all S messages
//! - Partial restart (0 < B < S): Verify remaining messages
//! - Normal operation (B=S): No verification, just append new messages

use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::collections::HashSet;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{IdbDatabase, IdbRequest, IdbTransactionMode};

use b4::messages::message::Message;
use b4::HttpB3Message;
use strand::serialization::StrandDeserialize;

use crate::protocol::board::local_storage::{LocalBoardStorage, StorageInfo};

/// Message metadata for duplicate detection
#[derive(Debug, Clone, Hash, Eq, PartialEq, Serialize, Deserialize)]
struct MessageMetadata {
    sender_pk: String,
    statement_kind: String,
    batch: i32,
    mix_number: i32,
}

/// Persistent metadata stored in IndexedDB
#[derive(Debug, Clone, Serialize, Deserialize)]
struct PersistentMetadata {
    /// Ordered list of message hashes (insertion order = local ID order)
    /// Position in this Vec is equivalent to SQLite AUTOINCREMENT ID
    /// Stored as Vec<u8> (serialized hashes) for serde compatibility
    hash_list: Vec<Vec<u8>>,

    /// Set of message identifiers for duplicate detection
    /// Equivalent to SQLite UNIQUE constraint
    metadata_set: HashSet<MessageMetadata>,
}

impl Default for PersistentMetadata {
    fn default() -> Self {
        PersistentMetadata {
            hash_list: Vec::new(),
            metadata_set: HashSet::new(),
        }
    }
}

/// Transient state (NOT persisted to IndexedDB)
#[derive(Debug, Default)]
struct TransientState {
    /// Last external_id from bulletin board (optimization within session only)
    /// Reset to -1 on page reload to force full re-fetch
    last_external_id: i64,

    /// Messages buffered between store_messages() and retrieve_messages() calls
    message_buffer: Vec<HttpB3Message>,
}

/// IndexedDB-backed storage for browser trustees
///
/// Provides persistent tamper-resistant storage using metadata-only approach.
/// Full messages are re-fetched from bulletin board and verified against stored hashes.
pub struct IndexedDbStorage {
    /// Database name
    db_name: String,

    /// Persistent metadata (synced with IndexedDB)
    persistent: RefCell<PersistentMetadata>,

    /// Transient state (in-memory only, not persisted)
    transient: RefCell<TransientState>,

    /// IndexedDB database handle (opened lazily)
    db: RefCell<Option<IdbDatabase>>,
}

// SAFETY: WASM is single-threaded, so RefCell is safe to share across "threads"
// (which don't actually exist in WASM). This allows IndexedDbStorage to implement
// LocalBoardStorage which requires Send + Sync.
unsafe impl Send for IndexedDbStorage {}
unsafe impl Sync for IndexedDbStorage {}

impl IndexedDbStorage {
    /// Create a new IndexedDB storage backend
    ///
    /// NOTE: This does NOT open the database. Call `init()` to load metadata.
    pub fn new(db_name: String) -> Self {
        IndexedDbStorage {
            db_name,
            persistent: RefCell::new(PersistentMetadata::default()),
            transient: RefCell::new(TransientState::default()),
            db: RefCell::new(None),
        }
    }

    /// Initialize storage: open IndexedDB and load persistent metadata
    ///
    /// This MUST be called before using the storage (async, call at session start).
    pub async fn init(&self) -> Result<()> {
        // Open or create IndexedDB database
        let db = self.open_database().await?;

        // Load persistent metadata from IndexedDB
        let metadata = self.load_metadata(&db).await?;

        // Store database handle and metadata
        *self.db.borrow_mut() = Some(db);
        *self.persistent.borrow_mut() = metadata;

        web_sys::console::log_1(&JsValue::from_str(&format!(
            "IndexedDB storage initialized: {} messages in store",
            self.persistent.borrow().hash_list.len()
        )));

        Ok(())
    }

    /// Persist current metadata to IndexedDB
    ///
    /// Call this after protocol steps to save state across page reloads.
    pub async fn persist(&self) -> Result<()> {
        let db = self
            .db
            .borrow()
            .clone()
            .ok_or_else(|| anyhow::anyhow!("Database not initialized. Call init() first"))?;

        let metadata = self.persistent.borrow().clone();
        self.save_metadata(&db, &metadata).await?;

        Ok(())
    }

    /// Clear all persistent storage (for testing)
    ///
    /// This resets both the in-memory state and the IndexedDB storage.
    pub async fn clear(&self) -> Result<()> {
        // Clear in-memory state
        *self.persistent.borrow_mut() = PersistentMetadata::default();
        *self.transient.borrow_mut() = TransientState::default();

        // Clear IndexedDB if database is open
        if let Some(db) = self.db.borrow().as_ref() {
            let transaction = db
                .transaction_with_str_and_mode("metadata", IdbTransactionMode::Readwrite)
                .map_err(|e| anyhow::anyhow!("Failed to create transaction: {:?}", e))?;

            let store = transaction
                .object_store("metadata")
                .map_err(|e| anyhow::anyhow!("Failed to access object store: {:?}", e))?;

            let delete_request = store
                .delete(&JsValue::from_str("persistent"))
                .map_err(|e| anyhow::anyhow!("Failed to delete metadata: {:?}", e))?;

            // Wait for deletion to complete
            use futures_channel::oneshot;
            use std::rc::Rc;

            let (sender, receiver) = oneshot::channel();
            let sender = Rc::new(RefCell::new(Some(sender)));

            let success_sender = sender.clone();
            let onsuccess = Closure::once(move |_event: web_sys::Event| {
                if let Some(tx) = success_sender.borrow_mut().take() {
                    let _ = tx.send(Ok(()));
                }
            });
            delete_request.set_onsuccess(Some(onsuccess.as_ref().unchecked_ref()));
            onsuccess.forget();

            let error_sender = sender.clone();
            let onerror = Closure::once(move |event: web_sys::Event| {
                if let Some(tx) = error_sender.borrow_mut().take() {
                    let _ = tx.send(Err(anyhow::anyhow!(
                        "Failed to delete metadata: {:?}",
                        event
                    )));
                }
            });
            delete_request.set_onerror(Some(onerror.as_ref().unchecked_ref()));
            onerror.forget();

            receiver
                .await
                .map_err(|_| anyhow::anyhow!("Channel closed"))?
                .map_err(|e| anyhow::anyhow!("Failed to clear storage: {}", e))?;

            web_sys::console::log_1(&JsValue::from_str("Cleared IndexedDB storage"));
        }

        Ok(())
    }

    /// Open IndexedDB database, creating object store if needed
    /// Open IndexedDB database, creating object store if needed
    async fn open_database(&self) -> Result<IdbDatabase> {
        use futures_channel::oneshot;
        use std::rc::Rc;
        let window = web_sys::window().ok_or_else(|| anyhow::anyhow!("No window"))?;
        let idb = window
            .indexed_db()
            .map_err(|_| anyhow::anyhow!("IndexedDB not supported"))?
            .ok_or_else(|| anyhow::anyhow!("IndexedDB not available"))?;

        let open_request = idb
            .open_with_u32(&self.db_name, 1)
            .map_err(|e| anyhow::anyhow!("Failed to open database: {:?}", e))?;

        // Handle database upgrade (create object store on first open)
        {
            let open_request_clone = open_request.clone();
            let onupgradeneeded =
                Closure::wrap(Box::new(move |event: web_sys::IdbVersionChangeEvent| {
                    web_sys::console::log_1(&JsValue::from_str("Creating object store..."));

                    if let Some(target) = event.target() {
                        if let Ok(request) = target.dyn_into::<IdbRequest>() {
                            if let Ok(db) =
                                request.result().and_then(|v| v.dyn_into::<IdbDatabase>())
                            {
                                let _ = db.create_object_store("metadata");
                            }
                        }
                    }
                }) as Box<dyn FnMut(_)>);

            open_request_clone.set_onupgradeneeded(Some(onupgradeneeded.as_ref().unchecked_ref()));
            onupgradeneeded.forget();
        }

        // Wait for database to open using event-based Promise
        let (sender, receiver) = oneshot::channel();
        let sender = Rc::new(RefCell::new(Some(sender)));

        // Success handler
        let success_sender = sender.clone();
        let onsuccess = Closure::once(move |event: web_sys::Event| {
            if let Some(target) = event.target() {
                if let Ok(request) = target.dyn_into::<IdbRequest>() {
                    if let Ok(result) = request.result() {
                        if let Some(tx) = success_sender.borrow_mut().take() {
                            let _ = tx.send(Ok(result));
                        }
                    }
                }
            }
        });
        open_request.set_onsuccess(Some(onsuccess.as_ref().unchecked_ref()));
        onsuccess.forget();

        // Error handler
        let error_sender = sender.clone();
        let onerror = Closure::once(move |event: web_sys::Event| {
            if let Some(tx) = error_sender.borrow_mut().take() {
                let _ = tx.send(Err(anyhow::anyhow!("IndexedDB open failed: {:?}", event)));
            }
        });
        open_request.set_onerror(Some(onerror.as_ref().unchecked_ref()));
        onerror.forget();

        // Wait for result
        let result = receiver
            .await
            .map_err(|_| anyhow::anyhow!("Channel closed"))?
            .map_err(|e| anyhow::anyhow!("Failed to open IndexedDB: {}", e))?;

        let db = result
            .dyn_into::<IdbDatabase>()
            .map_err(|e| anyhow::anyhow!("Invalid database object: {:?}", e))?;

        Ok(db)
    }

    /// Load metadata from IndexedDB
    async fn load_metadata(&self, db: &IdbDatabase) -> Result<PersistentMetadata> {
        use futures_channel::oneshot;
        use std::rc::Rc;

        let transaction = db
            .transaction_with_str_and_mode("metadata", IdbTransactionMode::Readonly)
            .map_err(|e| anyhow::anyhow!("Failed to create transaction: {:?}", e))?;

        let store = transaction
            .object_store("metadata")
            .map_err(|e| anyhow::anyhow!("Failed to access object store: {:?}", e))?;

        let get_request = store
            .get(&JsValue::from_str("persistent"))
            .map_err(|e| anyhow::anyhow!("Failed to get metadata: {:?}", e))?;

        // Event-based Promise
        let (sender, receiver) = oneshot::channel();
        let sender = Rc::new(RefCell::new(Some(sender)));

        let success_sender = sender.clone();
        let onsuccess = Closure::once(move |event: web_sys::Event| {
            if let Some(target) = event.target() {
                if let Ok(request) = target.dyn_into::<IdbRequest>() {
                    if let Ok(result) = request.result() {
                        if let Some(tx) = success_sender.borrow_mut().take() {
                            let _ = tx.send(Ok(result));
                        }
                    }
                }
            }
        });
        get_request.set_onsuccess(Some(onsuccess.as_ref().unchecked_ref()));
        onsuccess.forget();

        let error_sender = sender.clone();
        let onerror = Closure::once(move |event: web_sys::Event| {
            if let Some(tx) = error_sender.borrow_mut().take() {
                let _ = tx.send(Err(anyhow::anyhow!("Failed to get metadata: {:?}", event)));
            }
        });
        get_request.set_onerror(Some(onerror.as_ref().unchecked_ref()));
        onerror.forget();

        let result = receiver
            .await
            .map_err(|_| anyhow::anyhow!("Channel closed"))?
            .map_err(|e| anyhow::anyhow!("Failed to load metadata: {}", e))?;

        // If no metadata exists, return default
        if result.is_null() || result.is_undefined() {
            web_sys::console::log_1(&JsValue::from_str("No existing metadata, using defaults"));
            return Ok(PersistentMetadata::default());
        }

        // Deserialize from stored bytes
        let bytes = js_sys::Uint8Array::new(&result).to_vec();
        let metadata: PersistentMetadata = bincode::deserialize(&bytes)
            .map_err(|e| anyhow::anyhow!("Failed to deserialize metadata: {}", e))?;

        web_sys::console::log_1(&JsValue::from_str(&format!(
            "Loaded metadata: {} messages",
            metadata.hash_list.len()
        )));

        Ok(metadata)
    }

    /// Save metadata to IndexedDB
    async fn save_metadata(&self, db: &IdbDatabase, metadata: &PersistentMetadata) -> Result<()> {
        let transaction = db
            .transaction_with_str_and_mode("metadata", IdbTransactionMode::Readwrite)
            .map_err(|e| anyhow::anyhow!("Failed to create transaction: {:?}", e))?;

        let store = transaction
            .object_store("metadata")
            .map_err(|e| anyhow::anyhow!("Failed to access object store: {:?}", e))?;

        // Serialize metadata to bytes
        let bytes = bincode::serialize(metadata)
            .map_err(|e| anyhow::anyhow!("Failed to serialize metadata: {}", e))?;

        let uint8_array = js_sys::Uint8Array::from(&bytes[..]);

        let put_request = store
            .put_with_key(&uint8_array, &JsValue::from_str("persistent"))
            .map_err(|e| anyhow::anyhow!("Failed to put metadata: {:?}", e))?;

        // Event-based Promise
        use futures_channel::oneshot;
        use std::rc::Rc;

        let (sender, receiver) = oneshot::channel();
        let sender = Rc::new(RefCell::new(Some(sender)));

        let success_sender = sender.clone();
        let onsuccess = Closure::once(move |_event: web_sys::Event| {
            if let Some(tx) = success_sender.borrow_mut().take() {
                let _ = tx.send(Ok(()));
            }
        });
        put_request.set_onsuccess(Some(onsuccess.as_ref().unchecked_ref()));
        onsuccess.forget();

        let error_sender = sender.clone();
        let onerror = Closure::once(move |event: web_sys::Event| {
            if let Some(tx) = error_sender.borrow_mut().take() {
                let _ = tx.send(Err(anyhow::anyhow!("Failed to put metadata: {:?}", event)));
            }
        });
        put_request.set_onerror(Some(onerror.as_ref().unchecked_ref()));
        onerror.forget();

        receiver
            .await
            .map_err(|_| anyhow::anyhow!("Channel closed"))?
            .map_err(|e| anyhow::anyhow!("Failed to save metadata: {}", e))?;

        web_sys::console::log_1(&JsValue::from_str(&format!(
            "Saved metadata: {} messages",
            metadata.hash_list.len()
        )));

        Ok(())
    }

    /// Compute hash of message bytes
    fn compute_hash(msg: &HttpB3Message) -> Result<Vec<u8>> {
        let hash = strand::hash::hash_to_array(&msg.message)?;
        Ok(hash.to_vec())
    }

    /// Extract metadata from message for duplicate detection
    fn extract_metadata(msg: &HttpB3Message) -> Result<MessageMetadata> {
        let message = Message::strand_deserialize(&msg.message)?;

        Ok(MessageMetadata {
            sender_pk: message.sender.pk.to_der_b64_string()?,
            statement_kind: message.statement.get_kind().to_string(),
            batch: message.statement.get_batch_number().try_into()?,
            mix_number: message.statement.get_mix_number().try_into()?,
        })
    }

    /// Verify and store messages using the unified verification algorithm
    ///
    /// Given S (metadata store size) and B (local board size):
    /// - Verify first (S - B) messages match stored hashes
    /// - Append new messages beyond position S
    ///
    /// # Important: ID to Index Mapping
    ///
    /// Local IDs are 1-indexed (first message has id=1, like SQLite AUTOINCREMENT)
    /// hash_list is 0-indexed (hash_list[0] = hash of message with id=1)
    ///
    /// Therefore: hash_list[id - 1] = hash for message with local_id=id
    fn verify_and_store(
        &self,
        messages: &[HttpB3Message],
        local_board_id: i64,
        ignore_existing: bool,
    ) -> Result<(usize, usize)> {
        // Returns (verified_count, new_count)
        let mut persistent = self.persistent.borrow_mut();
        let mut transient = self.transient.borrow_mut();

        let S = persistent.hash_list.len() as i64;
        let B = local_board_id;

        // Normalize B: -1 is our initialization sentinel, treat as 0 for verification
        let B_normalized = if B == -1 { 0 } else { B };

        web_sys::console::log_1(&JsValue::from_str(&format!(
            "📊 VERIFICATION: S={} (store size), B={} (last_local_board_id, normalized={}), incoming={} messages",
            S, B, B_normalized, messages.len()
        )));

        // Invariant check: LocalBoard cannot be ahead of metadata store
        if B_normalized > S {
            bail!(
                "Corruption: LocalBoard has {} messages but metadata store only has {}",
                B_normalized,
                S
            );
        }

        // How many messages need verification against stored hashes?
        let verify_count = (S - B_normalized) as usize;

        if verify_count > 0 {
            web_sys::console::log_1(&JsValue::from_str(&format!(
                "🔍 Need to verify {} historical messages (hash_list positions {} through {})",
                verify_count,
                B_normalized,
                S - 1
            )));
        }

        if messages.len() < verify_count {
            bail!(
                "BB returned {} messages but we need {} to verify history (S={}, B={})",
                messages.len(),
                verify_count,
                S,
                B
            );
        }

        // Verify historical messages match our stored hashes
        // Messages are 1-indexed (first message has id=1), hash_list is 0-indexed
        // Special case: B=-1 means fresh start, verify from hash_list[0]
        // If B=1, we verify hash_list[1], hash_list[2], ... hash_list[S-1]
        // which corresponds to messages with id=2, id=3, ... id=S
        for i in 0..verify_count {
            let msg = &messages[i];

            // Calculate hash_list index for this message
            // hash_list is 0-indexed, IDs are 1-indexed
            // For B_normalized=0: verify hash_list[0], hash_list[1], ...
            // For B_normalized=1: verify hash_list[1], hash_list[2], ...
            let hash_index = B_normalized as usize + i;
            let next_id = B_normalized + (i as i64) + 1;

            web_sys::console::log_1(&JsValue::from_str(&format!(
                "  ✓ msg[{}] → hash_list[{}] (will be local_id={})",
                i, hash_index, next_id
            )));

            let expected_hash = &persistent.hash_list[hash_index];
            let actual_hash = Self::compute_hash(msg)?;

            if &actual_hash != expected_hash {
                bail!(
                    "🚨 TAMPER DETECTED! msg[{}] at hash_list[{}] (id={}) has wrong hash.\nExpected: {}\nActual: {}",
                    i,
                    hash_index,
                    next_id,
                    hex::encode(expected_hash),
                    hex::encode(&actual_hash)
                );
            }
        }

        if verify_count > 0 {
            web_sys::console::log_1(&JsValue::from_str(&format!(
                "✅ Verified {} messages (S={}, B={})",
                verify_count, S, B
            )));
            // Log prominent security verification message
            web_sys::console::log_1(&JsValue::from_str(&format!(
                "🔒 SECURITY: Verified {} historical message{} against stored hashes",
                verify_count,
                if verify_count == 1 { "" } else { "s" }
            )));
        }

        // Process new messages (beyond our stored history)
        let new_messages = &messages[verify_count..];
        if new_messages.len() > 0 {
            web_sys::console::log_1(&JsValue::from_str(&format!(
                "💾 Storing {} new messages (will assign local_ids {} through {})",
                new_messages.len(),
                S + 1,
                S + new_messages.len() as i64
            )));
        }

        let mut new_count = 0;
        for (idx, msg) in new_messages.iter().enumerate() {
            let hash = Self::compute_hash(msg)?;
            let metadata = Self::extract_metadata(msg)?;

            let new_local_id = S + 1 + idx as i64;

            // Check for duplicates
            if persistent.metadata_set.contains(&metadata) {
                if ignore_existing {
                    web_sys::console::log_1(&JsValue::from_str(&format!(
                        "  ⚠️  Skipping duplicate: {:?}",
                        metadata
                    )));
                    continue;
                } else {
                    bail!("Duplicate message: {:?}", metadata);
                }
            }

            web_sys::console::log_1(&JsValue::from_str(&format!(
                "  📝 new msg[{}] → hash_list[{}] (local_id={}, external_id={})",
                idx,
                persistent.hash_list.len(),
                new_local_id,
                msg.id
            )));

            // Add to persistent metadata
            persistent.hash_list.push(hash);
            persistent.metadata_set.insert(metadata);
            new_count += 1;

            // Update transient last_external_id (optimization for next fetch)
            transient.last_external_id = msg.id;
        }

        if new_count > 0 {
            web_sys::console::log_1(&JsValue::from_str(&format!(
                "✅ Stored {} new messages (store now has {} total)",
                new_count,
                persistent.hash_list.len()
            )));
        }

        if new_count > 0 {
            web_sys::console::log_1(&JsValue::from_str(&format!(
                "Added {} new messages to metadata store (total: {})",
                new_count,
                persistent.hash_list.len()
            )));
        }

        // Store messages in transient buffer for retrieve_messages()
        // transient.message_buffer = messages.to_vec();
        transient.message_buffer = new_messages.to_vec();

        Ok((verify_count, new_count))
    }
}

impl LocalBoardStorage for IndexedDbStorage {
    fn store_messages(&self, messages: &[HttpB3Message], _ignore_existing: bool) -> Result<()> {
        // NOTE: We don't know last_local_board_id here, so we assume B = S (normal case)
        // The verification will happen in retrieve_messages() where we have access to B

        // For now, just buffer the messages
        let mut transient = self.transient.borrow_mut();
        transient.message_buffer = messages.to_vec();

        // Update last_external_id optimization
        if let Some(last_msg) = messages.last() {
            transient.last_external_id = last_msg.id;
        }

        Ok(())
    }

    fn retrieve_messages(&self, last_local_board_id: i64) -> Result<Vec<(Message, i64)>> {
        // Clone messages to avoid borrow issues during verify_and_store
        let messages = {
            let transient = self.transient.borrow();
            transient.message_buffer.clone()
        };

        // Perform verification and update metadata
        let (verified_count, _new_count) =
            self.verify_and_store(&messages, last_local_board_id, false)?;

        // If we verified historical messages, this is a security-critical operation worth highlighting
        if verified_count > 0 {
            web_sys::console::log_1(&JsValue::from_str(&format!(
                "🛡️  Historical verification complete: {} message{} verified against tamper-resistant storage",
                verified_count,
                if verified_count == 1 { "" } else { "s" }
            )));
        }

        // Calculate how many messages we need to return
        // We need messages with local_id > last_local_board_id
        // These are at hash_list positions [last_local_board_id, last_local_board_id+1, ..., S-1]
        // which correspond to local_ids [last_local_board_id+1, last_local_board_id+2, ..., S]
        let S = {
            let persistent = self.persistent.borrow();
            persistent.hash_list.len() as i64
        };

        let B = last_local_board_id;
        let messages_to_return = (S - B) as usize;

        // The messages we need are at the END of the buffer (most recent)
        // because verify_and_store ensures buffer has [verified_messages, new_messages]
        let mut result: Vec<(Message, i64)> = messages
            .iter()
            .rev() // Start from the end
            .take(messages_to_return)
            .enumerate()
            .map(|(idx, msg)| {
                // idx=0 is the last message (local_id=S), idx=1 is second-to-last (local_id=S-1), etc.
                let local_id = S - (idx as i64);

                match Message::strand_deserialize(&msg.message) {
                    Ok(message) => Ok((message, local_id)),
                    Err(e) => Err(anyhow::anyhow!("Failed to deserialize message: {}", e)),
                }
            })
            .collect::<Result<Vec<_>>>()?;

        // Reverse back to ascending order by local_id
        result.reverse();
        Ok(result)
    }

    fn get_last_external_id(&self) -> Result<i64> {
        let transient = self.transient.borrow();
        Ok(transient.last_external_id)
    }

    fn get_storage_info(&self) -> Result<StorageInfo> {
        let persistent = self.persistent.borrow();
        let transient = self.transient.borrow();

        Ok(StorageInfo {
            backend_type: "IndexedDbStorage".to_string(),
            total_messages: persistent.hash_list.len() as i64,
            max_internal_id: persistent.hash_list.len() as i64,
            max_external_id: transient.last_external_id,
            extra_info: Some(format!(
                "metadata_set_size={}, db_name={}",
                persistent.metadata_set.len(),
                self.db_name
            )),
        })
    }
}
