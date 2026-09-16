// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! SQLite-backed predicate persistence (§6.2, M2): the native implementation
//! of the [`Persistence`] trait.
//!
//! Predicates are stored as their `Canonical` bytes in a single-column table whose
//! primary key is the bytes themselves, so `INSERT OR IGNORE` makes persistence
//! idempotent for identical predicates while still recording distinct ones. The
//! anti-rewrite decision lives in the board client's completeness gate (§6.3),
//! not `collides()` (that stays the datalog's job alone, §5.3); this module only
//! supplies the durable committed set the gate checks against.

use anyhow::Result;
use async_trait::async_trait;
use cryptography::utils::serialization::{Deserializable, Serializable};

use crate::board::persistence::Persistence;
use crate::board::transport::StagedRef;
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
        // The outbound record (§6.4): predicate bytes -> the handle that
        // publishes the staged message. Keyed by the predicate, so re-recording
        // the same post is idempotent.
        conn.execute(
            "CREATE TABLE IF NOT EXISTS own_posts (
                 bytes BLOB PRIMARY KEY,
                 staged_ref TEXT NOT NULL
             )",
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

    async fn load_own_posts(&self) -> Result<Vec<(Predicate, StagedRef)>> {
        let conn = self.conn.lock().expect("predicate store mutex poisoned");
        let mut statement = conn.prepare("SELECT bytes, staged_ref FROM own_posts")?;
        let rows = statement
            .query_map([], |row| {
                Ok((row.get::<_, Vec<u8>>(0)?, row.get::<_, String>(1)?))
            })?;
        let mut out = Vec::new();
        for row in rows {
            let (bytes, staged) = row?;
            let predicate = Predicate::deser(&bytes)
                .map_err(|e| anyhow::anyhow!("failed to deserialize own-post predicate: {e}"))?;
            out.push((predicate, StagedRef(staged)));
        }
        Ok(out)
    }

    async fn persist_own_post(&mut self, predicate: &Predicate, staged: &StagedRef) -> Result<()> {
        let bytes = predicate.ser();
        let conn = self.conn.lock().expect("predicate store mutex poisoned");
        conn.execute(
            "INSERT OR IGNORE INTO own_posts (bytes, staged_ref) VALUES (?1, ?2)",
            rusqlite::params![bytes, staged.0],
        )?;
        Ok(())
    }
}
