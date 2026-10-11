// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Tags the keys that `certs.json` has without their realm, so that Harvest
//! trusts them. Windmill beat schedules it on start and then every
//! `AWS_S3_JWKS_TAG_INTERVAL_SECS` seconds.

use crate::services::jwks;
use crate::tasks::purge_queue_archives::positive_setting;
use crate::types::error::Result;
use celery::error::TaskError;
use tracing::instrument;

pub const TAG_INTERVAL_ENV: &str = "AWS_S3_JWKS_TAG_INTERVAL_SECS";
pub const DEFAULT_TAG_INTERVAL_SECS: u64 = 3600;

/// Seconds between runs, from `AWS_S3_JWKS_TAG_INTERVAL_SECS`.
pub fn tag_interval_secs() -> anyhow::Result<u64> {
    positive_setting(
        TAG_INTERVAL_ENV,
        std::env::var(TAG_INTERVAL_ENV).ok().as_deref(),
        DEFAULT_TAG_INTERVAL_SECS,
    )
}

#[instrument(err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task(time_limit = 600, max_retries = 0, expires = 60)]
pub async fn tag_jwks_realms() -> Result<()> {
    jwks::tag_jwks_realms().await?;
    Ok(())
}
