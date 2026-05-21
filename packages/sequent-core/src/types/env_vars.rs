// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// Trustee authentication
pub const TRUSTEE_NAME: &str = "TRUSTEE_NAME";
pub const TRUSTEE_PSW: &str = "TRUSTEE_PSW";
pub const KC_TRUSTEE_CLIENT_SECRET: &str = "KC_TRUSTEE_CLIENT_SECRET";
pub const SUPER_ADMIN_TENANT_ID: &str = "SUPER_ADMIN_TENANT_ID";

// Braid protocol
pub const BRAID_B4_HEARTBEAT: &str = "BRAID_B4_HEARTBEAT";
pub const IGNORE_BOARDS: &str = "IGNORE_BOARDS";

pub const DEFAULT_BRAID_B4_HEARTBEAT_SECS: u32 = 5;
/// A session is considered inactive after this many missed heartbeat cycles.
/// Using 2 means one full interval of tolerance before flipping to NOT_ACTIVE.
pub const HEARTBEAT_GRACE_FACTOR: u32 = 2;

// B4 server
pub const B4_BIND: &str = "B4_BIND";
pub const B4_ALLOWED_ORIGINS: &str = "B4_ALLOWED_ORIGINS";
pub const ENV_SLUG: &str = "ENV_SLUG";

// PostgreSQL (B4 database)
pub const B4_PG_HOST: &str = "B4_PG_HOST";
pub const B4_PG_PORT: &str = "B4_PG_PORT";
pub const B4_PG_USER: &str = "B4_PG_USER";
pub const B4_PG_PASSWORD: &str = "B4_PG_PASSWORD";
pub const B4_PG_DATABASE: &str = "B4_PG_DATABASE";
pub const DATABASE_URL: &str = "DATABASE_URL";

// AWS / S3
pub const AWS_ENDPOINT_URL: &str = "AWS_ENDPOINT_URL";
pub const AWS_REGION: &str = "AWS_REGION";
pub const S3_BUCKET_NAME: &str = "S3_BUCKET_NAME";

// Default values for optional AWS / S3 env vars
pub const DEFAULT_S3_BUCKET: &str = "wbraid-messages";
pub const DEFAULT_S3_ENDPOINT: &str = "http://localhost:4566";
pub const DEFAULT_AWS_REGION: &str = "us-east-1";
