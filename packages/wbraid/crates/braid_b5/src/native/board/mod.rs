// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

/// Native HTTP client for bulletin board (B5)
pub mod http;
// Native-only (requires filesystem)
pub mod storage_sqlite;  

pub use http::{HttpB5, HttpB5BoardParams, HttpB5Index};
pub use storage_sqlite::SqliteStorage;
