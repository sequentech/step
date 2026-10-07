// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::celery_app::Queue;
use crate::services::database::get_queue_pool;
use crate::services::electoral_log_dead_letter::{
    apply_dead_letter_operation, DeadLetterOperation,
};
use crate::types::error::Result;
use anyhow::Context;
use celery::error::TaskError;
use tracing::{info, instrument};

/// Replay or discard dead-lettered electoral-log events, as a super administrator asked
/// on the Task Queues page. Harvest only enqueues tasks, so a worker applies it.
#[instrument(err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task(max_retries = 0)]
pub async fn manage_electoral_log_dead_letters(
    operation: DeadLetterOperation,
    message_ids: Vec<i64>,
    requested_by: String,
) -> Result<()> {
    let mut client = get_queue_pool()
        .await
        .get()
        .await
        .context("Error connecting to the task-queue database")?;
    let tx = client
        .transaction()
        .await
        .context("Error starting the dead-letter transaction")?;
    let found = apply_dead_letter_operation(
        &tx,
        Queue::ElectoralLogDeadLetter.queue_name(),
        Queue::ElectoralLogEvent.queue_name(),
        operation,
        &message_ids,
    )
    .await?;
    tx.commit()
        .await
        .context("Error committing the dead-letter operation")?;
    info!(
        "{requested_by} applied {operation} to {found} of {} dead-lettered electoral-log events",
        message_ids.len()
    );
    Ok(())
}
