// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::db::DbPool;

#[derive(Clone)]
pub struct AppState {
    pub db: DbPool,
}

// std::env::var("S3_BUCKET_NAME").unwrap_or_else(|_| "wbraid-messages".to_string());

impl AppState {
    pub fn new(db: DbPool) -> Self {
        Self { db }
    }
}
