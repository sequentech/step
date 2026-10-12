// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Posts the signing log outbox to the electoral log: beat sends it every
//! few seconds, and a signing step kicks it after it commits.

use crate::services::celery_app::get_celery_app;
use crate::services::database::get_hasura_pool;
use crate::services::signing::log::{
    post_signing_log_outboxes, BoardSigningLog, ElectoralLogKeys, EVENT_TIME_LIMIT,
};
use crate::types::error::Result;
use anyhow::anyhow;
use celery::error::TaskError;
use futures::FutureExt;
use std::future::Future;
use std::panic::AssertUnwindSafe;
use std::time::Duration;
use tokio::task::JoinHandle;
use tracing::{instrument, warn};

/// How long a kick may take to reach the broker.
const KICK_TIMEOUT: Duration = Duration::from_secs(5);

// Expires after a few beats: a later message posts the same entries. The
// time limit bounds a pass; each event has its own, shorter one.
#[instrument(err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task(max_retries = 0, expires = 30, time_limit = 300)]
pub async fn post_signing_log_outbox() -> Result<()> {
    let mut client = get_hasura_pool()
        .await
        .get()
        .await
        .map_err(|error| anyhow!("Error getting hasura client: {error}"))?;
    let mut board = BoardSigningLog::immudb();
    post_signing_log_outboxes(
        &mut client,
        &mut board,
        |_, _| ElectoralLogKeys::new(),
        EVENT_TIME_LIMIT,
    )
    .await?;
    Ok(())
}

async fn send_post_signing_log_outbox() -> anyhow::Result<()> {
    get_celery_app()
        .await
        .send_task(post_signing_log_outbox::new())
        .await
        .map(|_| ())
        .map_err(|error| anyhow!("{error:?}"))
}

/// Runs `send` in the background for at most `timeout`. A failure, a panic
/// or a timeout is only logged.
fn spawn_kick<F>(send: F, timeout: Duration) -> JoinHandle<()>
where
    F: Future<Output = anyhow::Result<()>> + Send + 'static,
{
    tokio::spawn(async move {
        match tokio::time::timeout(timeout, AssertUnwindSafe(send).catch_unwind()).await {
            Ok(Ok(Ok(()))) => {}
            Ok(Ok(Err(error))) => warn!("Error kicking the signing log outbox: {error:#}"),
            Ok(Err(_)) => warn!("Kicking the signing log outbox panicked"),
            Err(_) => warn!("Kicking the signing log outbox timed out"),
        }
    })
}

/// Asks a worker to post the outbox now, after a signing step commits.
/// Fire and forget: it returns at once, callers don't await the handle, and
/// it never fails or panics, even with the broker down; beat posts the
/// entries anyway.
pub fn kick_signing_log_outbox() -> JoinHandle<()> {
    spawn_kick(send_post_signing_log_outbox(), KICK_TIMEOUT)
}

#[cfg(test)]
#[path = "signing_log_outbox_tests.rs"]
mod tests;
