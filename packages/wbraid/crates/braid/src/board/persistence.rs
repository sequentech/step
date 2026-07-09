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
//! The trait is async so the wasm IndexedDB backend (M3) fits the same shape as
//! native SQLite (M2). M1 uses [`NoOpPersistence`]: nothing is persisted, and a
//! restart therefore relies entirely on re-fetching from b4 (§6.3).

use anyhow::Result;
use async_trait::async_trait;

use crate::messages::predicate::Predicate;
use cryptography::utils::serialization::{VDeserializable, VSerializable};

/// One persistence backend, two media (§6.2): SQLite (native, M2) / IndexedDB
/// (wasm, M3), with [`NoOpPersistence`] for M1.
#[async_trait]
pub trait Persistence: Send + Sync {
    /// Load the persisted predicate set on restart (§6.3). NoOp returns empty.
    async fn load(&self) -> Result<Vec<Predicate>>;
    /// Persist a predicate digest before its body is admitted to memory (§6.2).
    async fn persist(&mut self, predicate: &Predicate) -> Result<()>;
}

/// No-op persistence (M1): nothing is persisted; restart loads nothing.
pub struct NoOpPersistence;

#[async_trait]
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
/// `Mutex` to satisfy the `Send + Sync` bound on [`Persistence`]. The calls are
/// synchronous and never hold the lock across an `.await`.
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
#[async_trait]
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
