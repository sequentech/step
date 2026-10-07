// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Setup of the environment's task-queue database, run by its owner when the environment
//! is provisioned or upgraded.

use crate::services::celery_app::Queue;
use crate::services::database::generate_queue_pool;
use anyhow::{Context, Result};
use pgmq_broker::setup::{setup, Installation, PgmqInstallation, Roles};
use std::time::Duration;
use tracing::warn;

/// The role of Windmill's workers and Beat.
pub const WORKER_ROLE_ENV: &str = "QUEUE_DB_WORKER_ROLE";
/// The role of the services that only enqueue tasks: Harvest and Keycloak.
pub const PRODUCER_ROLE_ENV: &str = "QUEUE_DB_PRODUCER_ROLE";
/// The role of read-only queue inspection.
pub const READER_ROLE_ENV: &str = "QUEUE_DB_READER_ROLE";
/// The database may still be starting when the setup runs.
const CONNECT_ATTEMPTS: u32 = 30;
const CONNECT_RETRY_DELAY: Duration = Duration::from_secs(2);

fn required_env(name: &str) -> Result<String> {
    let value = std::env::var(name).with_context(|| format!("missing env var {name}"))?;
    anyhow::ensure!(!value.is_empty(), "{name} must not be empty");
    Ok(value)
}

/// Install PGMQ, record the environment, create every queue and grant the roles access,
/// connected with `QUEUE_DB__*` as the database owner.
pub async fn set_up_task_queue_database() -> Result<PgmqInstallation> {
    let environment = required_env("ENV_SLUG")?;
    let roles = Roles {
        worker: required_env(WORKER_ROLE_ENV)?,
        producer: required_env(PRODUCER_ROLE_ENV)?,
        reader: required_env(READER_ROLE_ENV)?,
    };
    let pool = generate_queue_pool().await?;
    let mut attempt = 1;
    let mut client = loop {
        match pool.get().await {
            Ok(client) => break client,
            Err(error) if attempt < CONNECT_ATTEMPTS => {
                warn!("Task-queue database not reachable yet (attempt {attempt}): {error}");
                attempt += 1;
                tokio::time::sleep(CONNECT_RETRY_DELAY).await;
            }
            Err(error) => {
                return Err(error).context("Error connecting to the task-queue database");
            }
        }
    };
    let tx = client
        .transaction()
        .await
        .context("Error starting the task-queue setup")?;
    let queues = Queue::all_names();
    let installation = setup(
        &tx,
        &Installation {
            environment: &environment,
            queues: &queues,
            roles: Some(&roles),
        },
    )
    .await
    .context("Error setting up the task-queue database")?;
    tx.commit()
        .await
        .context("Error committing the task-queue setup")?;
    Ok(installation)
}
