// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Posts the audit log outbox to the tenants' and the election events' logs:
//! beat sends it every few seconds, and a change kicks it after it commits.

use crate::services::audit_log_writer::{post_audit_log_outboxes, KeycloakUsernames};
use crate::services::celery_app::try_get_celery_app;
use crate::services::database::get_hasura_pool;
use crate::types::error::Result;
use anyhow::anyhow;
use celery::error::TaskError;
use std::sync::OnceLock;
use std::time::Duration;
use tokio::task::JoinHandle;
use tracing::{instrument, warn};

/// How long a kick may take to reach the task-queue database.
const KICK_TIMEOUT: Duration = Duration::from_secs(5);

static USERNAMES: OnceLock<KeycloakUsernames> = OnceLock::new();

// Expires after a few beats: a later message posts the same entries.
#[instrument(err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task(max_retries = 0, expires = 30, time_limit = 300)]
pub async fn post_audit_log_outbox() -> Result<()> {
    let mut client = get_hasura_pool()
        .await
        .get()
        .await
        .map_err(|error| anyhow!("Error getting hasura client: {error}"))?;
    post_audit_log_outboxes(
        &mut client,
        USERNAMES.get_or_init(KeycloakUsernames::default),
    )
    .await?;
    Ok(())
}

async fn send_post_audit_log_outbox() -> anyhow::Result<()> {
    try_get_celery_app()
        .await?
        .send_task(post_audit_log_outbox::new())
        .await?;
    Ok(())
}

/// Asks a worker to post the outbox now, after a change commits. Fire and
/// forget: it returns at once and never fails, even with the task-queue
/// database down; beat posts the entries anyway.
pub fn kick_audit_log_outbox() -> JoinHandle<()> {
    tokio::spawn(async {
        match tokio::time::timeout(KICK_TIMEOUT, send_post_audit_log_outbox()).await {
            Ok(Ok(())) => {}
            Ok(Err(error)) => warn!("Error kicking the audit log outbox: {error:#}"),
            Err(_) => warn!("Kicking the audit log outbox timed out"),
        }
    })
}
