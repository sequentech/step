// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

/// Native HTTP client for bulletin board (B3/B4)
#[cfg(feature = "native")]
pub mod http;

/// Storage implementations
pub mod storage_noop; // Available in WASM (temporary, for testing)

#[cfg(feature = "native")]
pub mod storage_sqlite; // Native-only (requires filesystem)

// Re-export HTTP types (native-only)
#[cfg(feature = "native")]
pub use http::{HttpB3, HttpB3BoardParams, HttpB3Index};

// Re-export storage types
pub use storage_noop::NoOpStorage;

#[cfg(feature = "native")]
pub use storage_sqlite::SqliteStorage;
