// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Election-event import: upload the template JSON, then
//! `import_election_event`. The mutation itself only *enqueues* the import
//! (it returns as soon as the celery task is scheduled), so the caller
//! must poll the `task_execution` it hands back.

use anyhow::{Context, Result};
use graphql_client::GraphQLQuery;

use crate::hasura::HasuraClient;
use crate::provision::tasks::poll_task_execution;
use crate::provision::upload::upload_document;
use crate::types::hasura::*;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/graphql/schema.json",
    query_path = "src/graphql/import_election_event.graphql",
    response_derives = "Debug,Clone,Deserialize,Serialize"
)]
pub struct ImportElectionEvent;

/// Uploads `template_bytes` and imports it into `tenant_id`, returning the
/// new election event id once the import has actually completed.
pub async fn import_election_event(
    client: &HasuraClient,
    http: &reqwest::Client,
    tenant_id: &str,
    template_bytes: &[u8],
) -> Result<String> {
    let document_id = upload_document(
        client,
        http,
        "election-event.json",
        "application/json",
        None,
        template_bytes,
    )
    .await
    .context("failed to upload the election-event template")?;

    let import_variables = import_election_event::Variables {
        tenant_id: tenant_id.to_string(),
        document_id,
        check_only: None,
    };
    let import_data = client
        .data_or_bail::<ImportElectionEvent>(import_variables)
        .await
        .context("failed to start the election-event import")?;
    let imported = import_data
        .import_election_event
        .ok_or_else(|| anyhow::anyhow!("import_election_event returned no data"))?;
    if let Some(error) = imported.error {
        anyhow::bail!("election-event import rejected: {error}");
    }
    let election_event_id = imported
        .id
        .ok_or_else(|| anyhow::anyhow!("import_election_event returned no election event id"))?;

    if let Some(task_execution) = imported.task_execution {
        poll_task_execution(client, &task_execution.id)
            .await
            .with_context(|| {
                format!("election event {election_event_id} import did not complete")
            })?;
    }

    Ok(election_event_id)
}
