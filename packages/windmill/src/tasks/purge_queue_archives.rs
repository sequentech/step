// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Keeps the task queues' archives to a retention period. Every processed message is
//! archived with its outcome, so the archives show what ran and how it ended; old entries
//! are deleted. The dead-letter queue's archive is never purged.

use crate::services::celery_app::Queue;
use crate::services::database::get_queue_pool;
use crate::types::error::Result;
use anyhow::Context;
use celery::error::TaskError;
use std::time::Duration;
use strum::IntoEnumIterator;
use tracing::{info, instrument};

pub const PURGE_INTERVAL_ENV: &str = "QUEUE_ARCHIVE_PURGE_INTERVAL_SECS";
pub const RETENTION_ENV: &str = "QUEUE_ARCHIVE_RETENTION_HOURS";
pub const DEFAULT_PURGE_INTERVAL_SECS: u64 = 3600;
pub const DEFAULT_RETENTION_HOURS: u64 = 7 * 24;
/// Rows deleted per statement, so that a purge never holds many row locks at once.
const PURGE_BATCH: i64 = 5_000;

/// Seconds between purges, from `QUEUE_ARCHIVE_PURGE_INTERVAL_SECS`.
pub fn purge_interval_secs() -> anyhow::Result<u64> {
    positive_setting(
        PURGE_INTERVAL_ENV,
        std::env::var(PURGE_INTERVAL_ENV).ok().as_deref(),
        DEFAULT_PURGE_INTERVAL_SECS,
    )
}

/// How long archived messages are kept, from `QUEUE_ARCHIVE_RETENTION_HOURS`.
pub fn retention() -> anyhow::Result<Duration> {
    let hours = positive_setting(
        RETENTION_ENV,
        std::env::var(RETENTION_ENV).ok().as_deref(),
        DEFAULT_RETENTION_HOURS,
    )?;
    hours
        .checked_mul(3600)
        .map(Duration::from_secs)
        .with_context(|| format!("{RETENTION_ENV} is too large, got {hours}"))
}

fn positive_setting(name: &str, value: Option<&str>, default: u64) -> anyhow::Result<u64> {
    match value.map(str::trim).filter(|value| !value.is_empty()) {
        None => Ok(default),
        Some(value) => value
            .parse::<u64>()
            .ok()
            .filter(|parsed| *parsed > 0)
            .with_context(|| format!("{name} must be a positive integer, got {value:?}")),
    }
}

/// The queues whose archives are purged.
fn purged_queues() -> impl Iterator<Item = Queue> {
    Queue::iter().filter(|queue| *queue != Queue::ElectoralLogDeadLetter)
}

/// Delete archived messages older than the retention period. Windmill beat schedules it
/// every `QUEUE_ARCHIVE_PURGE_INTERVAL_SECS` seconds.
#[instrument(err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task(time_limit = 600, max_retries = 0, expires = 60)]
pub async fn purge_queue_archives() -> Result<()> {
    let retention = retention()?;
    let client = get_queue_pool()
        .await
        .get()
        .await
        .context("Error connecting to the task-queue database")?;
    for queue in purged_queues() {
        let deleted =
            pgmq_broker::purge_archive(&**client, queue.queue_name(), retention, PURGE_BATCH)
                .await
                .with_context(|| format!("Error purging the archive of {}", queue.queue_name()))?;
        if deleted > 0 {
            info!(
                "Purged {deleted} archived messages of {} older than {}h",
                queue.queue_name(),
                retention.as_secs() / 3600
            );
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_default_and_reject_non_positive_values() {
        assert_eq!(positive_setting("X", None, 7).unwrap(), 7);
        assert_eq!(positive_setting("X", Some("  "), 7).unwrap(), 7);
        assert_eq!(positive_setting("X", Some(" 30 "), 7).unwrap(), 30);
        for invalid in ["0", "-1", "1h", "2.5"] {
            let error = positive_setting("X", Some(invalid), 7)
                .unwrap_err()
                .to_string();
            assert!(error.contains("X must be a positive integer"), "{error}");
        }
    }

    #[test]
    fn the_dead_letter_archive_is_never_purged() {
        let purged: Vec<Queue> = purged_queues().collect();
        assert!(!purged.contains(&Queue::ElectoralLogDeadLetter));
        assert_eq!(purged.len(), Queue::iter().count() - 1);
    }
}
