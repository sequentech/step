// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Shared Axum state: the database pool, the S3 client and the bucket name,
//! read from the environment once at startup (native only).

use crate::db::common::BoardDb;
use aws_sdk_s3::Client as S3Client;

/// Names the S3 bucket message bodies are staged in.
pub const S3_BUCKET_NAME_ENV: &str = "S3_BUCKET_NAME";
const DEFAULT_S3_BUCKET_NAME: &str = "wbraid-messages";

#[derive(Clone)]
pub struct AppState {
    pub db: BoardDb,
    pub s3_client: S3Client,
    pub bucket_name: String,
}

impl AppState {
    pub fn new(db: BoardDb, s3_client: S3Client, bucket_name: String) -> Self {
        Self {
            db,
            s3_client,
            bucket_name,
        }
    }

    /// The bucket from [`S3_BUCKET_NAME_ENV`], falling back to the default when
    /// unset or empty (compose substitutes an empty string for a variable
    /// missing from its env file).
    pub fn from_env(db: BoardDb, s3_client: S3Client) -> Self {
        let bucket_name = std::env::var(S3_BUCKET_NAME_ENV)
            .ok()
            .filter(|name| !name.trim().is_empty())
            .unwrap_or_else(|| DEFAULT_S3_BUCKET_NAME.to_string());
        Self::new(db, s3_client, bucket_name)
    }
}
