// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Predicate persistence (§6.2 of `crates/braid/v0.6_spec.md`).
//!
//! Persisted predicates are the b4-sourced (looped-back) ones; their sole job is
//! **anti-rewrite** — they pin, irreversibly, which `H(body)` b4 has committed to
//! for each slot (§6.3). Persistence is NOT what prevents re-execution (that is
//! update-first + loop-back), and it is NOT a defense against b4 dropping a
//! message (that is availability).
//!
//! The trait is async (and `?Send`, spec Option B) so the wasm IndexedDB backend
//! (M3) — whose `JsFuture`s are `!Send` — fits the same shape as native SQLite
//! (M2). M1 uses [`NoOpPersistence`]: nothing is persisted, and a restart
//! therefore relies entirely on re-fetching from b4 (§6.3).

use anyhow::Result;
use async_trait::async_trait;

use crate::messages::predicate::Predicate;
// Predicate (de)serialization for the durable backends (native SQLite / wasm
// IndexedDB); unused by the in-memory `NoOpPersistence`.
#[cfg(any(feature = "native", feature = "wasm-core"))]
use cryptography::utils::serialization::{VDeserializable, VSerializable};

/// One persistence backend, two media (§6.2): SQLite (native, M2) / IndexedDB
/// (wasm, M3), with [`NoOpPersistence`] for M1. `?Send` (Option B) so the wasm
/// backend's `!Send` futures fit; native parallelism bounds the concrete type
/// with `+ Sync` at the rayon call site, not the trait.
#[async_trait(?Send)]
pub trait Persistence {
    /// Load the persisted predicate set on restart (§6.3). NoOp returns empty.
    async fn load(&self) -> Result<Vec<Predicate>>;
    /// Persist a predicate digest before its body is admitted to memory (§6.2).
    async fn persist(&mut self, predicate: &Predicate) -> Result<()>;
}

/// No-op persistence (M1): nothing is persisted; restart loads nothing.
pub struct NoOpPersistence;

#[async_trait(?Send)]
impl Persistence for NoOpPersistence {
    async fn load(&self) -> Result<Vec<Predicate>> {
        Ok(Vec::new())
    }
    async fn persist(&mut self, _predicate: &Predicate) -> Result<()> {
        Ok(())
    }
}

/// SQLite-backed persistence (native, M2).
///
/// Predicates are stored as their `VSer` bytes in a single-column table whose
/// primary key is the bytes themselves, so `INSERT OR IGNORE` makes
/// [`persist`](SqlitePersistence::persist) idempotent for identical predicates
/// (a re-statement of the same slot) while still recording distinct ones. The
/// anti-rewrite decision lives in the board client's boundary `collides()`
/// check (§6.3); persistence only supplies the durable committed set.
///
/// rusqlite's `Connection` is `Send` but not `Sync`, so it is wrapped in a
/// `Mutex`. The [`Persistence`] trait itself is `?Send`, but the native test
/// harnesses share `&BoardClient` across rayon threads, which requires the
/// concrete persistence type to be `Sync`. The `Mutex` satisfies that bound;
/// the calls are synchronous and never hold the lock across an `.await`.
#[cfg(feature = "native")]
pub struct SqlitePersistence {
    conn: std::sync::Mutex<rusqlite::Connection>,
}

#[cfg(feature = "native")]
impl SqlitePersistence {
    /// Open (creating if absent) a SQLite database at `path` and ensure the
    /// `predicates` table exists.
    pub fn open(path: impl AsRef<std::path::Path>) -> Result<Self> {
        let conn = rusqlite::Connection::open(path)?;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS predicates (bytes BLOB PRIMARY KEY)",
            [],
        )?;
        Ok(Self {
            conn: std::sync::Mutex::new(conn),
        })
    }
}

#[cfg(feature = "native")]
#[async_trait(?Send)]
impl Persistence for SqlitePersistence {
    async fn load(&self) -> Result<Vec<Predicate>> {
        let conn = self.conn.lock().expect("predicate store mutex poisoned");
        let mut statement = conn.prepare("SELECT bytes FROM predicates")?;
        let rows = statement.query_map([], |row| row.get::<_, Vec<u8>>(0))?;
        let mut predicates = Vec::new();
        for row in rows {
            let bytes = row?;
            let predicate = Predicate::deser(&bytes)
                .map_err(|e| anyhow::anyhow!("failed to deserialize persisted predicate: {e}"))?;
            predicates.push(predicate);
        }
        Ok(predicates)
    }

    async fn persist(&mut self, predicate: &Predicate) -> Result<()> {
        let bytes = predicate.ser();
        let conn = self.conn.lock().expect("predicate store mutex poisoned");
        conn.execute(
            "INSERT OR IGNORE INTO predicates (bytes) VALUES (?1)",
            rusqlite::params![bytes],
        )?;
        Ok(())
    }
}

/// IndexedDB-backed persistence (wasm, M3).
///
/// Mirrors [`SqlitePersistence`]: predicates are stored in an object store keyed
/// by their `VSer` bytes (an out-of-line binary key), so `put` is idempotent for
/// identical predicates. Anti-rewrite remains the board client's boundary
/// `collides()` check (§6.3); this only supplies the durable committed set.
///
/// The web-sys handles and `JsFuture`s are `!Send`, which is why the
/// [`Persistence`] seam is `?Send` (spec Option B).
#[cfg(feature = "wasm-core")]
pub struct IndexedDbPersistence {
    db: indexed_db_futures::prelude::IdbDatabase,
}

/// The single object store holding predicate bytes.
#[cfg(feature = "wasm-core")]
const PREDICATE_STORE: &str = "predicates";

#[cfg(feature = "wasm-core")]
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

#[cfg(feature = "wasm-core")]
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
