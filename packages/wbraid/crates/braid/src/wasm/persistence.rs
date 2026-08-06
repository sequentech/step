// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! IndexedDB-backed predicate persistence (§6.2, M3): the wasm implementation
//! of the [`Persistence`] trait.
//!
//! Mirrors [`SqlitePersistence`](crate::native::persistence::SqlitePersistence):
//! predicates are stored in an object store keyed by their `VSer` bytes (an
//! out-of-line binary key), so `put` is idempotent for identical predicates.
//! Anti-rewrite remains the board client's completeness gate (§6.3), not
//! `collides()` (that stays the datalog's job alone, §5.3); this module only
//! supplies the durable committed set the gate checks against.
//!
//! The web-sys handles and `JsFuture`s are `!Send`, which is why the
//! [`Persistence`] seam is `?Send` (spec Option B).

use anyhow::Result;
use async_trait::async_trait;
use cryptography::utils::serialization::{VDeserializable, VSerializable};

use crate::board::persistence::Persistence;
use crate::board::transport::StagedRef;
use crate::messages::predicate::Predicate;

/// IndexedDB-backed persistence (wasm, M3).
pub struct IndexedDbPersistence {
    db: indexed_db_futures::prelude::IdbDatabase,
}

/// The object store holding predicate bytes (the committed set, §6.2).
const PREDICATE_STORE: &str = "predicates";
/// The object store holding the outbound record (§6.4): predicate bytes keyed to
/// the handle that publishes the staged message.
const OWN_POST_STORE: &str = "own_posts";

impl IndexedDbPersistence {
    /// Open (creating if absent) the IndexedDB database `db_name`, ensuring the
    /// predicate object store exists.
    pub async fn open(db_name: &str) -> Result<Self> {
        use indexed_db_futures::prelude::*;
        use indexed_db_futures::IdbVersionChangeEvent;
        use wasm_bindgen::JsValue;

        // Version 2 adds the own-post store (§6.4); the upgrade handler creates
        // whichever stores are missing, so an existing v1 database is extended
        // rather than rebuilt.
        let mut request = IdbDatabase::open_u32(db_name, 2)
            .map_err(|e| anyhow::anyhow!("failed to open IndexedDB: {:?}", e))?;
        request.set_on_upgrade_needed(Some(
            |evt: &IdbVersionChangeEvent| -> std::result::Result<(), JsValue> {
                for store in [PREDICATE_STORE, OWN_POST_STORE] {
                    if evt
                        .db()
                        .object_store_names()
                        .find(|n| n == store)
                        .is_none()
                    {
                        evt.db().create_object_store(store)?;
                    }
                }
                Ok(())
            },
        ));
        let db = request
            .await
            .map_err(|e| anyhow::anyhow!("failed to open IndexedDB: {:?}", e))?;
        Ok(Self { db })
    }
}

#[async_trait(?Send)]
impl Persistence for IndexedDbPersistence {
    async fn load(&self) -> Result<Vec<Predicate>> {
        use indexed_db_futures::prelude::*;

        let tx = self
            .db
            .transaction_on_one(PREDICATE_STORE)
            .map_err(|e| anyhow::anyhow!("idb read tx: {:?}", e))?;
        let store = tx
            .object_store(PREDICATE_STORE)
            .map_err(|e| anyhow::anyhow!("idb object store: {:?}", e))?;
        let all = store
            .get_all()
            .map_err(|e| anyhow::anyhow!("idb get_all: {:?}", e))?
            .await
            .map_err(|e| anyhow::anyhow!("idb get_all await: {:?}", e))?;

        let mut predicates = Vec::with_capacity(all.length() as usize);
        for value in all.iter() {
            let bytes = js_sys::Uint8Array::new(&value).to_vec();
            let predicate = Predicate::deser(&bytes)
                .map_err(|e| anyhow::anyhow!("failed to deserialize persisted predicate: {e}"))?;
            predicates.push(predicate);
        }
        Ok(predicates)
    }

    async fn persist(&mut self, predicate: &Predicate) -> Result<()> {
        use indexed_db_futures::prelude::*;

        let bytes = predicate.ser();
        let array = js_sys::Uint8Array::from(bytes.as_slice());
        let tx = self
            .db
            .transaction_on_one_with_mode(PREDICATE_STORE, IdbTransactionMode::Readwrite)
            .map_err(|e| anyhow::anyhow!("idb write tx: {:?}", e))?;
        let store = tx
            .object_store(PREDICATE_STORE)
            .map_err(|e| anyhow::anyhow!("idb object store: {:?}", e))?;
        // Key == value == the predicate bytes: identical predicates overwrite
        // in place (idempotent), distinct ones get distinct keys.
        store
            .put_key_val_owned(array.clone(), &array)
            .map_err(|e| anyhow::anyhow!("idb put: {:?}", e))?;
        tx.await
            .into_result()
            .map_err(|e| anyhow::anyhow!("idb write tx commit: {:?}", e))?;
        Ok(())
    }

    async fn load_own_posts(&self) -> Result<Vec<(Predicate, StagedRef)>> {
        use indexed_db_futures::prelude::*;

        let tx = self
            .db
            .transaction_on_one(OWN_POST_STORE)
            .map_err(|e| anyhow::anyhow!("idb read tx: {:?}", e))?;
        let store = tx
            .object_store(OWN_POST_STORE)
            .map_err(|e| anyhow::anyhow!("idb object store: {:?}", e))?;
        let all = store
            .get_all()
            .map_err(|e| anyhow::anyhow!("idb get_all: {:?}", e))?
            .await
            .map_err(|e| anyhow::anyhow!("idb get_all await: {:?}", e))?;

        let mut out = Vec::with_capacity(all.length() as usize);
        for value in all.iter() {
            let bytes = js_sys::Uint8Array::new(&value).to_vec();
            out.push(decode_own_post(&bytes)?);
        }
        Ok(out)
    }

    async fn persist_own_post(&mut self, predicate: &Predicate, staged: &StagedRef) -> Result<()> {
        use indexed_db_futures::prelude::*;

        let key = js_sys::Uint8Array::from(predicate.ser().as_slice());
        let value = js_sys::Uint8Array::from(encode_own_post(predicate, staged).as_slice());
        let tx = self
            .db
            .transaction_on_one_with_mode(OWN_POST_STORE, IdbTransactionMode::Readwrite)
            .map_err(|e| anyhow::anyhow!("idb write tx: {:?}", e))?;
        let store = tx
            .object_store(OWN_POST_STORE)
            .map_err(|e| anyhow::anyhow!("idb object store: {:?}", e))?;
        // Keyed by the predicate bytes, so re-recording the same post overwrites
        // in place (idempotent).
        store
            .put_key_val_owned(key, &value)
            .map_err(|e| anyhow::anyhow!("idb put: {:?}", e))?;
        tx.await
            .into_result()
            .map_err(|e| anyhow::anyhow!("idb write tx commit: {:?}", e))?;
        Ok(())
    }
}

/// Encode an own-post entry as `[handle len: u32 LE][handle][predicate bytes]`.
///
/// IndexedDB's `get_all` returns values without their keys, so the value carries
/// both halves rather than relying on the key. Hand-rolled instead of `VSer`
/// because the handle is a `String`, which the protocol serializer does not
/// cover — and this encoding is local storage only, never on the wire, so it
/// needs no canonicality guarantee.
fn encode_own_post(predicate: &Predicate, staged: &StagedRef) -> Vec<u8> {
    let handle = staged.0.as_bytes();
    let mut out = Vec::with_capacity(4 + handle.len() + 64);
    out.extend((handle.len() as u32).to_le_bytes());
    out.extend(handle);
    out.extend(predicate.ser());
    out
}

fn decode_own_post(bytes: &[u8]) -> Result<(Predicate, StagedRef)> {
    if bytes.len() < 4 {
        anyhow::bail!("own-post entry too short");
    }
    let len = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as usize;
    let rest = &bytes[4..];
    if rest.len() < len {
        anyhow::bail!("own-post entry handle length {len} exceeds the entry");
    }
    let handle = std::str::from_utf8(&rest[..len])
        .map_err(|e| anyhow::anyhow!("own-post handle is not UTF-8: {e}"))?;
    let predicate = Predicate::deser(&rest[len..])
        .map_err(|e| anyhow::anyhow!("failed to deserialize own-post predicate: {e}"))?;
    Ok((predicate, StagedRef(handle.to_string())))
}
