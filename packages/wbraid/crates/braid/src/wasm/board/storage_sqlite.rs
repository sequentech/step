// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! SQLite storage backend for LocalBoard (WASM with OPFS)
//!
//! Provides persistent, tamper-resistant message storage using SQLite WASM
//! with OPFS (Origin Private File System) backend. This implementation uses
//! the same SQL schema as the native SqliteStorage for compatibility.
//!
//! # Architecture
//!
//! - **sqlite-wasm-rs**: Pure Rust SQLite bindings for wasm32-unknown-unknown
//! - **OPFS SAH Pool VFS**: Synchronous OPFS access via createSyncAccessHandle()
//! - **Worker Context**: OPFS VFS requires Dedicated Worker (provided by rayon)
//! - **Same Schema**: Identical to native storage_sqlite.rs for consistency
//!
//! # Initialization
//!
//! The OPFS VFS must be installed asynchronously before opening databases:
//! ```rust
//! use sqlite_wasm_rs::sahpool_vfs::{install as install_opfs_sahpool, OpfsSAHPoolCfg};
//! 
//! // Call once during app initialization
//! install_opfs_sahpool(&OpfsSAHPoolCfg::default(), true).await.unwrap();
//! ```

use std::cell::RefCell;
use std::ffi::{CStr, CString};
use std::ptr;
use anyhow::{Result, anyhow};

use b4::messages::message::Message;
use b4::HttpB3Message;
use strand::serialization::StrandDeserialize;

use crate::protocol::board::local_storage::LocalBoardStorage;

/// SQLite WASM storage with OPFS backend
///
/// # Security Model
///
/// Identical to native SqliteStorage:
/// - **Locally-controlled IDs**: SQLite AUTOINCREMENT ensures insertion order
/// - **Tamper-resistant**: UNIQUE constraints prevent duplicates
/// - **Append-only**: Messages cannot be deleted or reordered
///
/// # OPFS VFS
///
/// Uses sahpool_vfs (Sync Access Handle Pool) which provides:
/// - Synchronous file I/O in Workers via createSyncAccessHandle()
/// - Full durability with proper fsync semantics
/// - Single connection per database (OPFS limitation)
///
/// # Thread Safety
///
/// Uses RefCell instead of Mutex because WASM runs single-threaded.
/// Manually implements Send + Sync (safe in WASM context).
pub struct SqliteStorage {
    /// Database file name (stored in OPFS root)
    db_name: String,
    /// Cached database connection (opened lazily)
    connection: RefCell<Option<*mut sqlite_wasm_rs::sqlite3>>,
}

// SAFETY: SqliteStorage is safe to Send/Sync in WASM because:
// - WASM runs single-threaded on the main thread
// - RefCell provides interior mutability without actual thread contention
// - Rayon workers run in separate WASM instances with separate databases
// - sqlite-wasm-rs is compiled with SQLITE_THREADSAFE=0 (single-threaded)
unsafe impl Send for SqliteStorage {}
unsafe impl Sync for SqliteStorage {}

impl SqliteStorage {
    /// Create a new SQLite WASM storage backend
    ///
    /// # Parameters
    ///
    /// - `db_name`: Database filename (e.g., "board.db")
    ///
    /// # Note
    ///
    /// The OPFS VFS must be installed before creating SqliteStorage:
    /// ```rust
    /// install_opfs_sahpool(&OpfsSAHPoolCfg::default(), true).await?;
    /// ```
    pub fn new(db_name: String) -> Self {
        SqliteStorage {
            db_name,
            connection: RefCell::new(None),
        }
    }

    /// Get or create a connection to the SQLite database
    ///
    /// Creates the MESSAGES table if it doesn't exist.
    ///
    /// # Security Model
    ///
    /// Identical schema to native storage_sqlite.rs:
    /// - `id`: AUTOINCREMENT PRIMARY KEY - locally-controlled, determines processing order
    /// - `external_id`: Bulletin board's ID (UNIQUE) - optimization only, no security impact
    /// - `UNIQUE(sender_pk, statement_kind, batch, mix_number)`: Prevents duplicate messages
    fn get_connection(&self) -> Result<*mut sqlite_wasm_rs::sqlite3> {
        let mut conn_ref = self.connection.borrow_mut();
        
        if let Some(db) = *conn_ref {
            // Return existing connection
            return Ok(db);
        }

        // Open new connection with OPFS VFS
        let mut db: *mut sqlite_wasm_rs::sqlite3 = ptr::null_mut();
        let db_path = CString::new(self.db_name.as_str())?;
        
        let rc = unsafe {
            sqlite_wasm_rs::sqlite3_open_v2(
                db_path.as_ptr(),
                &mut db as *mut _,
                sqlite_wasm_rs::SQLITE_OPEN_READWRITE | sqlite_wasm_rs::SQLITE_OPEN_CREATE,
                ptr::null(), // Use default VFS (should be opfs-sahpool if installed)
            )
        };

        if rc != sqlite_wasm_rs::SQLITE_OK {
            let err_msg = if db.is_null() {
                "Failed to open database".to_string()
            } else {
                unsafe {
                    let msg_ptr = sqlite_wasm_rs::sqlite3_errmsg(db);
                    if msg_ptr.is_null() {
                        "Unknown error".to_string()
                    } else {
                        CStr::from_ptr(msg_ptr).to_string_lossy().to_string()
                    }
                }
            };
            
            if !db.is_null() {
                unsafe { sqlite_wasm_rs::sqlite3_close(db) };
            }
            
            return Err(anyhow!("sqlite3_open_v2 failed ({}): {}", rc, err_msg));
        }

        // Create MESSAGES table (same schema as native)
        let create_table_sql = CString::new(
            "CREATE TABLE IF NOT EXISTS MESSAGES(\
                id INTEGER PRIMARY KEY AUTOINCREMENT, \
                external_id INT8 NOT NULL UNIQUE, \
                message BLOB NOT NULL, \
                sender_pk TEXT NOT NULL, \
                statement_kind TEXT NOT NULL, \
                batch INT4 NOT NULL, \
                mix_number INT4 NOT NULL, \
                UNIQUE(sender_pk, statement_kind, batch, mix_number)\
            )"
        )?;

        let rc = unsafe {
            sqlite_wasm_rs::sqlite3_exec(
                db,
                create_table_sql.as_ptr(),
                None,
                ptr::null_mut(),
                ptr::null_mut(),
            )
        };

        if rc != sqlite_wasm_rs::SQLITE_OK {
            let err_msg = unsafe {
                let msg_ptr = sqlite_wasm_rs::sqlite3_errmsg(db);
                CStr::from_ptr(msg_ptr).to_string_lossy().to_string()
            };
            unsafe { sqlite_wasm_rs::sqlite3_close(db) };
            return Err(anyhow!("Failed to create MESSAGES table ({}): {}", rc, err_msg));
        }

        *conn_ref = Some(db);
        Ok(db)
    }

    /// Execute a SQL statement with error handling
    fn exec_sql(&self, db: *mut sqlite_wasm_rs::sqlite3, sql: &str) -> Result<()> {
        let sql_cstr = CString::new(sql)?;
        let rc = unsafe {
            sqlite_wasm_rs::sqlite3_exec(
                db,
                sql_cstr.as_ptr(),
                None,
                ptr::null_mut(),
                ptr::null_mut(),
            )
        };

        if rc != sqlite_wasm_rs::SQLITE_OK {
            let err_msg = unsafe {
                let msg_ptr = sqlite_wasm_rs::sqlite3_errmsg(db);
                CStr::from_ptr(msg_ptr).to_string_lossy().to_string()
            };
            return Err(anyhow!("SQL execution failed ({}): {}", rc, err_msg));
        }

        Ok(())
    }
}

impl LocalBoardStorage for SqliteStorage {
    fn store_messages(&self, messages: &[HttpB3Message], ignore_existing: bool) -> Result<()> {
        if messages.is_empty() {
            return Ok(());
        }

        let db = self.get_connection()?;

        // Choose INSERT statement based on ignore_existing flag
        let sql = if ignore_existing {
            "INSERT OR IGNORE INTO MESSAGES(external_id, message, sender_pk, statement_kind, batch, mix_number) \
             VALUES(?1, ?2, ?3, ?4, ?5, ?6)"
        } else {
            "INSERT INTO MESSAGES(external_id, message, sender_pk, statement_kind, batch, mix_number) \
             VALUES(?1, ?2, ?3, ?4, ?5, ?6)"
        };

        let sql_cstr = CString::new(sql)?;
        let mut stmt: *mut sqlite_wasm_rs::sqlite3_stmt = ptr::null_mut();

        let rc = unsafe {
            sqlite_wasm_rs::sqlite3_prepare_v2(
                db,
                sql_cstr.as_ptr(),
                -1,
                &mut stmt as *mut _,
                ptr::null_mut(),
            )
        };

        if rc != sqlite_wasm_rs::SQLITE_OK {
            let err_msg = unsafe {
                let msg_ptr = sqlite_wasm_rs::sqlite3_errmsg(db);
                CStr::from_ptr(msg_ptr).to_string_lossy().to_string()
            };
            return Err(anyhow!("Failed to prepare INSERT statement ({}): {}", rc, err_msg));
        }

        // Begin transaction
        self.exec_sql(db, "BEGIN TRANSACTION")?;

        let mut insert_count = 0;
        for m in messages {
            // Verify schema version compatibility
            if m.version != b4::get_schema_version() {
                self.exec_sql(db, "ROLLBACK")?;
                unsafe { sqlite_wasm_rs::sqlite3_finalize(stmt) };
                return Err(anyhow!(
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

            // Bind parameters
            unsafe {
                // ?1: external_id (i64)
                sqlite_wasm_rs::sqlite3_bind_int64(stmt, 1, m.id);
                
                // ?2: message (BLOB)
                sqlite_wasm_rs::sqlite3_bind_blob(
                    stmt,
                    2,
                    m.message.as_ptr() as *const _,
                    m.message.len() as i32,
                    sqlite_wasm_rs::SQLITE_TRANSIENT(),
                );
                
                // ?3: sender_pk (TEXT)
                let sender_pk_cstr = CString::new(sender_pk.as_str())?;
                sqlite_wasm_rs::sqlite3_bind_text(
                    stmt,
                    3,
                    sender_pk_cstr.as_ptr(),
                    -1,
                    sqlite_wasm_rs::SQLITE_TRANSIENT(),
                );
                
                // ?4: statement_kind (TEXT)
                let kind_cstr = CString::new(kind.as_str())?;
                sqlite_wasm_rs::sqlite3_bind_text(
                    stmt,
                    4,
                    kind_cstr.as_ptr(),
                    -1,
                    sqlite_wasm_rs::SQLITE_TRANSIENT(),
                );
                
                // ?5: batch (INT4)
                sqlite_wasm_rs::sqlite3_bind_int(stmt, 5, batch);
                
                // ?6: mix_number (INT4)
                sqlite_wasm_rs::sqlite3_bind_int(stmt, 6, mix_number);
            }

            // Execute INSERT
            let rc = unsafe { sqlite_wasm_rs::sqlite3_step(stmt) };
            
            if rc != sqlite_wasm_rs::SQLITE_DONE {
                let err_msg = unsafe {
                    let msg_ptr = sqlite_wasm_rs::sqlite3_errmsg(db);
                    CStr::from_ptr(msg_ptr).to_string_lossy().to_string()
                };
                self.exec_sql(db, "ROLLBACK")?;
                unsafe { sqlite_wasm_rs::sqlite3_finalize(stmt) };
                return Err(anyhow!("INSERT failed ({}): {}", rc, err_msg));
            }

            insert_count += 1;

            // Reset statement for next iteration
            unsafe { sqlite_wasm_rs::sqlite3_reset(stmt) };
        }

        // Commit transaction
        self.exec_sql(db, "END TRANSACTION")?;
        
        unsafe { sqlite_wasm_rs::sqlite3_finalize(stmt) };

        web_sys::console::log_1(&format!("store_messages: inserted {} messages", insert_count).into());

        Ok(())
    }

    fn retrieve_messages(&self, last_local_board_id: i64) -> Result<Vec<(Message, i64)>> {
        let db = self.get_connection()?;

        // SECURITY CRITICAL: ORDER BY id ASC ensures messages are processed in the
        // order established by our local AUTOINCREMENT ID, not the bulletin board's order
        let sql = CString::new(
            "SELECT id, message FROM MESSAGES WHERE id > ?1 ORDER BY id ASC"
        )?;

        let mut stmt: *mut sqlite_wasm_rs::sqlite3_stmt = ptr::null_mut();

        let rc = unsafe {
            sqlite_wasm_rs::sqlite3_prepare_v2(
                db,
                sql.as_ptr(),
                -1,
                &mut stmt as *mut _,
                ptr::null_mut(),
            )
        };

        if rc != sqlite_wasm_rs::SQLITE_OK {
            let err_msg = unsafe {
                let msg_ptr = sqlite_wasm_rs::sqlite3_errmsg(db);
                CStr::from_ptr(msg_ptr).to_string_lossy().to_string()
            };
            return Err(anyhow!("Failed to prepare SELECT statement ({}): {}", rc, err_msg));
        }

        // Bind last_local_board_id parameter
        unsafe {
            sqlite_wasm_rs::sqlite3_bind_int64(stmt, 1, last_local_board_id);
        }

        let mut messages = Vec::new();

        loop {
            let rc = unsafe { sqlite_wasm_rs::sqlite3_step(stmt) };

            if rc == sqlite_wasm_rs::SQLITE_ROW {
                // Read row data
                let id = unsafe { sqlite_wasm_rs::sqlite3_column_int64(stmt, 0) };
                
                let message_blob = unsafe {
                    let blob_ptr = sqlite_wasm_rs::sqlite3_column_blob(stmt, 1);
                    let blob_len = sqlite_wasm_rs::sqlite3_column_bytes(stmt, 1);
                    
                    if blob_ptr.is_null() || blob_len == 0 {
                        Vec::new()
                    } else {
                        std::slice::from_raw_parts(blob_ptr as *const u8, blob_len as usize).to_vec()
                    }
                };

                // Deserialize message
                let message = Message::strand_deserialize(&message_blob)?;
                messages.push((message, id));
                
            } else if rc == sqlite_wasm_rs::SQLITE_DONE {
                break;
            } else {
                let err_msg = unsafe {
                    let msg_ptr = sqlite_wasm_rs::sqlite3_errmsg(db);
                    CStr::from_ptr(msg_ptr).to_string_lossy().to_string()
                };
                unsafe { sqlite_wasm_rs::sqlite3_finalize(stmt) };
                return Err(anyhow!("SELECT failed ({}): {}", rc, err_msg));
            }
        }

        unsafe { sqlite_wasm_rs::sqlite3_finalize(stmt) };

        Ok(messages)
    }

    fn get_last_external_id(&self) -> Result<i64> {
        let db = self.get_connection()?;

        let sql = CString::new("SELECT MAX(external_id) FROM MESSAGES")?;
        let mut stmt: *mut sqlite_wasm_rs::sqlite3_stmt = ptr::null_mut();

        let rc = unsafe {
            sqlite_wasm_rs::sqlite3_prepare_v2(
                db,
                sql.as_ptr(),
                -1,
                &mut stmt as *mut _,
                ptr::null_mut(),
            )
        };

        if rc != sqlite_wasm_rs::SQLITE_OK {
            let err_msg = unsafe {
                let msg_ptr = sqlite_wasm_rs::sqlite3_errmsg(db);
                CStr::from_ptr(msg_ptr).to_string_lossy().to_string()
            };
            return Err(anyhow!("Failed to prepare MAX query ({}): {}", rc, err_msg));
        }

        let rc = unsafe { sqlite_wasm_rs::sqlite3_step(stmt) };
        
        let max_external_id = if rc == sqlite_wasm_rs::SQLITE_ROW {
            unsafe {
                let col_type = sqlite_wasm_rs::sqlite3_column_type(stmt, 0);
                if col_type == sqlite_wasm_rs::SQLITE_NULL {
                    -1
                } else {
                    sqlite_wasm_rs::sqlite3_column_int64(stmt, 0)
                }
            }
        } else {
            -1
        };

        unsafe { sqlite_wasm_rs::sqlite3_finalize(stmt) };

        Ok(max_external_id)
    }
}

impl Drop for SqliteStorage {
    fn drop(&mut self) {
        if let Some(db) = *self.connection.borrow_mut() {
            unsafe {
                sqlite_wasm_rs::sqlite3_close(db);
            }
        }
    }
}
