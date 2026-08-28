// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Polls `sequent_backend_tasks_execution` — the generic status row every
//! celery-backed mutation (`insertTenant`, `import_election_event`) creates
//! and updates asynchronously. Neither mutation's own response means the
//! underlying work finished; both hand back a `task_execution` to poll here.

use std::time::Duration;

use anyhow::{bail, Result};
use graphql_client::GraphQLQuery;
use tokio::time::sleep;

use crate::hasura::HasuraClient;
use crate::types::hasura::*;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/graphql/schema.json",
    query_path = "src/graphql/get_task_execution.graphql",
    response_derives = "Debug,Clone,Deserialize,Serialize"
)]
pub struct GetTaskExecution;

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(120);
const DEFAULT_POLL_INTERVAL: Duration = Duration::from_secs(2);

pub async fn poll_task_execution(client: &HasuraClient, task_execution_id: &str) -> Result<()> {
    poll_task_execution_with(
        client,
        task_execution_id,
        DEFAULT_TIMEOUT,
        DEFAULT_POLL_INTERVAL,
    )
    .await
}

pub async fn poll_task_execution_with(
    client: &HasuraClient,
    task_execution_id: &str,
    timeout: Duration,
    poll_interval: Duration,
) -> Result<()> {
    let start = tokio::time::Instant::now();
    loop {
        let variables = get_task_execution::Variables {
            task_execution_id: task_execution_id.to_string(),
        };
        let data = client.data_or_bail::<GetTaskExecution>(variables).await?;
        let Some(row) = data.sequent_backend_tasks_execution.first() else {
            bail!("task execution {task_execution_id} not found");
        };

        match row.execution_status.as_str() {
            "SUCCESS" => return Ok(()),
            "FAILED" => bail!("task execution {task_execution_id} failed"),
            "CANCELLED" => bail!("task execution {task_execution_id} was cancelled"),
            _ => {
                if start.elapsed() >= timeout {
                    bail!(
                        "timed out waiting for task execution {task_execution_id} \
                         to complete"
                    );
                }
                sleep(poll_interval).await;
            }
        }
    }
}
