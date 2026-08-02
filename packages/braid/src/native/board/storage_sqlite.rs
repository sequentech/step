// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! SQLite storage backend for LocalBoard (native only)
//!
//! Provides persistent, tamper-resistant message storage using SQLite.
//! Messages are stored with locally-controlled auto-increment IDs that
//! establish immutable ordering, preventing bulletin board manipulation.

use anyhow::Result;
use rusqlite::{params, Connection};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::PathBuf;
use std::time::Instant;

use b4::messages::message::Message;
use b4::HttpB3Message;
use strand::serialization::StrandDeserialize;

use crate::protocol::board::local_storage::LocalBoardStorage;

/// SQLite-based persistent storage
///
/// # Security Model
///
/// - **Locally-controlled IDs**: SQLite AUTOINCREMENT ensures insertion order
/// - **Tamper-resistant**: UNIQUE constraints prevent duplicates
/// - **Append-only**: Messages cannot be deleted or reordered
///
/// # Optional Blob Storage
///
/// Large message artifacts can be stored in the filesystem (`blob_store`)
/// instead of the database to improve performance. The database stores
/// metadata and small messages inline.
pub struct SqliteStorage {
    store_path: PathBuf,
    blob_store: Option<PathBuf>,
}

impl SqliteStorage {
    /// Create a new SQLite storage backend
    ///
    /// # Parameters
    ///
    /// - `store_path`: Path to SQLite database file
    /// - `blob_store`: Optional directory for large message blobs
    pub fn new(store_path: PathBuf, blob_store: Option<PathBuf>) -> Self {
        SqliteStorage {
            store_path,
            blob_store,
        }
    }

    /// Get a connection to the SQLite database
    ///
    /// Creates the MESSAGES table if it doesn't exist.
    ///
    /// # Security Model
    ///
    /// - `id`: AUTOINCREMENT PRIMARY KEY - locally-controlled, determines processing order
    /// - `external_id`: Bulletin board's ID (UNIQUE) - optimization only, no security impact
    /// - `UNIQUE(sender_pk, statement_kind, batch, mix_number)`: Prevents duplicate messages
    ///
    /// See https://www.sqlite.org/autoinc.html
    fn get_connection(&self) -> Result<Connection> {
        let connection = Connection::open(&self.store_path)?;

        connection.execute(
            "CREATE TABLE if not exists MESSAGES(\
                id INTEGER PRIMARY KEY AUTOINCREMENT, \
                external_id INT8 NOT NULL UNIQUE, \
                message BLOB NOT NULL, \
                sender_pk TEXT NOT NULL, \
                statement_kind TEXT NOT NULL, \
                batch INT4 NOT NULL, \
                mix_number INT4 NOT NULL, \
                UNIQUE(sender_pk, statement_kind, batch, mix_number)\
            )",
            [],
        )?;

        Ok(connection)
    }
}

impl LocalBoardStorage for SqliteStorage {
    fn store_messages(&self, messages: &[HttpB3Message], ignore_existing: bool) -> Result<()> {
        let now = Instant::now();

        // Ensure blob store directory exists if configured
        if let Some(blob_store) = &self.blob_store {
            if !blob_store.exists() {
                fs::create_dir_all(blob_store)?;
            }
        }

        let connection = self.get_connection()?;

        // Choose INSERT statement based on ignore_existing flag
        // The trustee triggers a full message update via the RETRIEVE_ALL_MESSAGES_PERIOD,
        // so we can ignore duplicates in that case.
        let sql = if ignore_existing {
            "INSERT OR IGNORE INTO MESSAGES(external_id, message, sender_pk, statement_kind, batch, mix_number) \
             VALUES(?1, ?2, ?3, ?4, ?5, ?6)"
        } else {
            "INSERT INTO MESSAGES(external_id, message, sender_pk, statement_kind, batch, mix_number) \
             VALUES(?1, ?2, ?3, ?4, ?5, ?6)"
        };

        let mut statement = connection.prepare(sql)?;

        connection.execute("BEGIN TRANSACTION", [])?;

        for m in messages {
            // Verify schema version compatibility
            if m.version != b4::get_schema_version() {
                return Err(anyhow::anyhow!(
                    "Mismatched schema version: {} != {}",
                    m.version,
                    b4::get_schema_version()
                ));
            }

            // Deserialize to extract metadata
            let message = Message::strand_deserialize(&m.message)?;
            let sender_pk = message.sender.pk.to_der_b64_string()?;
            let kind = message.statement.get_kind().to_string();
            let batch: i32 = message.statement.get_batch_number().try_into()?;
            let mix_number: i32 = message.statement.get_mix_number().try_into()?;

            if let Some(blob_store) = &self.blob_store {
                // Store message bytes in filesystem blob store
                let name = format!("{}-{}-{}-{}", kind, sender_pk, batch, mix_number);
                let path = blob_store.join(name.replace("/", ":"));

                if !path.exists() {
                    let mut file = File::create(&path)?;
                    file.write_all(&m.message)?;
                    tracing::info!(
                        "store_messages: wrote {} bytes to {:?}",
                        m.message.len(),
                        path
                    );
                }

                // Store metadata only (empty message bytes)
                statement.execute(params![m.id, vec![], sender_pk, kind, batch, mix_number])?;
            } else {
                // Store message bytes inline in database
                statement.execute(params![m.id, m.message, sender_pk, kind, batch, mix_number])?;
            }
        }

        connection.execute("END TRANSACTION", [])?;
        drop(statement);

        if !messages.is_empty() {
            tracing::info!(
                "store_messages: inserted {} messages in {}ms",
                messages.len(),
                now.elapsed().as_millis()
            );
        }

        Ok(())
    }

    fn retrieve_messages(&self, last_local_board_id: i64) -> Result<Vec<(Message, i64)>> {
        let connection = self.get_connection()?;

        // SECURITY CRITICAL: ORDER BY id ASC ensures messages are processed in the
        // order established by our local AUTOINCREMENT ID, not the bulletin board's order
        let mut stmt = connection.prepare(
            "SELECT id, message, sender_pk, statement_kind, batch, mix_number \
             FROM MESSAGES \
             WHERE id > ?1 \
             ORDER BY id ASC",
        )?;

        let rows = stmt.query_map([last_local_board_id], |row| {
            Ok(SqliteStoreMessageRow {
                id: row.get(0)?,
                message: row.get(1)?,
                sender_pk: row.get(2)?,
                kind: row.get(3)?,
                batch: row.get(4)?,
                mix_number: row.get(5)?,
            })
        })?;

        let messages: Result<Vec<(Message, i64)>> = rows
            .map(|mr| {
                let row = mr?;
                let id = row.id;

                let message = if let Some(blob_store) = &self.blob_store {
                    // Read message bytes from filesystem blob store
                    let name = format!(
                        "{}-{}-{}-{}",
                        row.kind, row.sender_pk, row.batch, row.mix_number
                    );
                    let path = blob_store.join(name.replace("/", ":"));

                    if !path.exists() {
                        return Err(anyhow::anyhow!("Blob file not found: {:?}", path));
                    }

                    let mut file = File::open(&path)?;
                    let mut buffer = Vec::new();
                    let bytes_read = file.read_to_end(&mut buffer)?;

                    tracing::info!(
                        "retrieve_messages: read {} bytes from {:?}",
                        bytes_read,
                        path
                    );

                    Message::strand_deserialize(&buffer)?
                } else {
                    // Read message bytes from database
                    Message::strand_deserialize(&row.message)?
                };

                Ok((message, id))
            })
            .collect();

        messages
    }

    fn get_last_external_id(&self) -> Result<i64> {
        let connection = self.get_connection()?;

        let external_last_id = connection
            .query_row("SELECT MAX(external_id) FROM messages", [], |row| {
                row.get(0)
            })
            .unwrap_or(-1);

        Ok(external_last_id)
    }

    fn get_storage_info(&self) -> Result<crate::protocol::board::local_storage::StorageInfo> {
        use crate::protocol::board::storage_schema::GET_STORAGE_INFO_SQL;

        let connection = self.get_connection()?;

        let (total_messages, max_internal_id, max_external_id) =
            connection.query_row(GET_STORAGE_INFO_SQL, [], |row| {
                Ok((
                    row.get::<_, i64>(0).unwrap_or(0),
                    row.get::<_, Option<i64>>(1).unwrap_or(None).unwrap_or(0),
                    row.get::<_, Option<i64>>(2).unwrap_or(None).unwrap_or(0),
                ))
            })?;

        let extra_info = if let Some(blob_store) = &self.blob_store {
            Some(format!(
                "Database: {:?}, Blob store: {:?}",
                self.store_path, blob_store
            ))
        } else {
            Some(format!("Database: {:?}", self.store_path))
        };

        Ok(crate::protocol::board::local_storage::StorageInfo {
            backend_type: "SqliteStorage (Native)".to_string(),
            total_messages,
            max_internal_id,
            max_external_id,
            extra_info,
        })
    }
}

/// Row structure for SQLite query results
struct SqliteStoreMessageRow {
    id: i64,
    message: Vec<u8>,
    sender_pk: String,
    kind: String,
    batch: i32,
    mix_number: i32,
}
