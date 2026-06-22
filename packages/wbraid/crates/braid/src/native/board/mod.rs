// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

/// Native HTTP client for bulletin board (B4)
pub mod http;
// Native-only (requires filesystem)
pub mod storage_sqlite;  

pub use http::{HttpB4, HttpB4BoardParams, HttpB4Index};
pub use storage_sqlite::SqliteStorage;
