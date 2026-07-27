// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! IndexedDB-backed predicate persistence (§6.2, M3): the wasm implementation
//! of the [`Persistence`] trait.
//!
//! Mirrors [`SqlitePersistence`](crate::native::persistence::SqlitePersistence):
//! predicates are stored in an object store keyed by their `VSer` bytes (an
//! out-of-line binary key), so `put` is idempotent for identical predicates.
//! Anti-rewrite remains the board client's boundary `collides()` check (§6.3);
//! this only supplies the durable committed set.
//!
//! The web-sys handles and `JsFuture`s are `!Send`, which is why the
//! [`Persistence`] seam is `?Send` (spec Option B).

use anyhow::Result;
use async_trait::async_trait;
use cryptography::utils::serialization::{VDeserializable, VSerializable};

use crate::board::persistence::Persistence;
use crate::messages::predicate::Predicate;

/// IndexedDB-backed persistence (wasm, M3).
pub struct IndexedDbPersistence {
    db: indexed_db_futures::prelude::IdbDatabase,
}

/// The single object store holding predicate bytes.
const PREDICATE_STORE: &str = "predicates";

impl IndexedDbPersistence {
    /// Open (creating if absent) the IndexedDB database `db_name`, ensuring the
    /// predicate object store exists.
    pub async fn open(db_name: &str) -> Result<Self> {
        use indexed_db_futures::prelude::*;
        use indexed_db_futures::IdbVersionChangeEvent;
        use wasm_bindgen::JsValue;

        let mut request = IdbDatabase::open_u32(db_name, 1)
            .map_err(|e| anyhow::anyhow!("failed to open IndexedDB: {:?}", e))?;
        request.set_on_upgrade_needed(Some(
            |evt: &IdbVersionChangeEvent| -> std::result::Result<(), JsValue> {
                if evt
                    .db()
                    .object_store_names()
                    .find(|n| n == PREDICATE_STORE)
                    .is_none()
                {
                    evt.db().create_object_store(PREDICATE_STORE)?;
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
}
