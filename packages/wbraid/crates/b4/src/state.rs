// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use aws_sdk_s3::Client as S3Client;
use sqlx::SqlitePool;

#[derive(Clone)]
pub struct AppState {
    pub db: SqlitePool,
    pub s3_client: S3Client,
    pub bucket_name: String,
}

impl AppState {
    pub fn new(db: SqlitePool, s3_client: S3Client) -> Self {
        let bucket_name =
            std::env::var("S3_BUCKET_NAME").unwrap_or_else(|_| "wbraid-messages".to_string());

        Self {
            db,
            s3_client,
            bucket_name,
        }
    }
}
