// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! WASM-specific board implementations

pub mod http;
pub mod storage_browser;

#[cfg(feature = "sqlite-wasm-rs")]
pub mod storage_sqlite;

pub use http::{WasmHttpBoard, WasmHttpBoardFactory, WasmHttpBoardParams};
pub use storage_browser::BrowserStorage;

#[cfg(feature = "sqlite-wasm-rs")]
pub use storage_sqlite::SqliteStorage;

/// Initialize OPFS VFS for SQLite WASM
///
/// This must be called once before creating any SqliteStorage instances.
/// Uses the sahpool_vfs (Sync Access Handle Pool) backend which provides
/// synchronous OPFS access in Worker contexts.
///
/// # Example
///
/// ```rust
/// use braid::wasm::board::init_sqlite_opfs;
///
/// // Call during app initialization
/// init_sqlite_opfs().await?;
/// ```
#[cfg(feature = "sqlite-wasm-rs")]
pub async fn init_sqlite_opfs() -> Result<(), wasm_bindgen::JsValue> {
    use sqlite_wasm_rs::sahpool_vfs::{install as install_opfs_sahpool, OpfsSAHPoolCfg};
    
    // Install OPFS SAH Pool VFS as default
    // NOTE: This REQUIRES Web Worker context - createSyncAccessHandle() only works in Workers
    // If called from main thread, will fail with "NotSupported" error
    install_opfs_sahpool(&OpfsSAHPoolCfg::default(), true)
        .await
        .map_err(|e| wasm_bindgen::JsValue::from_str(&format!("Failed to initialize OPFS VFS: {:?}", e)))?;
    
    web_sys::console::log_1(&"SQLite OPFS VFS initialized".into());
    
    Ok(())
}

