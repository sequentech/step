// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::db::DbPool;
use sequent_core::types::env_vars as ev;

use aws_sdk_s3::Client as S3Client;

#[derive(Clone)]
pub struct AppState {
    pub db: DbPool,
    pub s3_client: S3Client,
    pub bucket_name: String,
}

impl AppState {
    pub fn new(db: DbPool, s3_client: S3Client) -> Self {
        let bucket_name =
            std::env::var(ev::S3_BUCKET_NAME).unwrap_or_else(|_| ev::DEFAULT_S3_BUCKET.to_string());

        Self {
            db,
            s3_client,
            bucket_name,
        }
    }
}
