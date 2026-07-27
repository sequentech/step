// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! SQLite-backed predicate persistence (§6.2, M2): the native implementation
//! of the [`Persistence`] trait.
//!
//! Predicates are stored as their `VSer` bytes in a single-column table whose
//! primary key is the bytes themselves, so `INSERT OR IGNORE` makes persistence
//! idempotent for identical predicates while still recording distinct ones. The
//! anti-rewrite decision lives in the board client's boundary `collides()` check
//! (§6.3); this only supplies the durable committed set.

use anyhow::Result;
use async_trait::async_trait;
use cryptography::utils::serialization::{VDeserializable, VSerializable};

use crate::board::persistence::Persistence;
use crate::messages::predicate::Predicate;

/// SQLite-backed persistence (native, M2).
///
/// rusqlite's `Connection` is `Send` but not `Sync`, so it is wrapped in a
/// `Mutex`. The [`Persistence`] trait itself is `?Send`, but the native test
/// harnesses share `&BoardClient` across rayon threads, which requires the
/// concrete persistence type to be `Sync`. The `Mutex` satisfies that bound;
/// the calls are synchronous and never hold the lock across an `.await`.
pub struct SqlitePersistence {
    conn: std::sync::Mutex<rusqlite::Connection>,
}

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
