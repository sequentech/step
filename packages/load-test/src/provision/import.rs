// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Election-event import: `get_upload_url` -> PUT the template JSON to the
//! returned pre-signed URL -> `import_election_event`. The mutation itself
//! only *enqueues* the import (it returns as soon as the celery task is
//! scheduled), so the caller must poll the `task_execution` it hands back.

use anyhow::{bail, Context, Result};
use graphql_client::GraphQLQuery;

use crate::hasura::HasuraClient;
use crate::provision::tasks::poll_task_execution;
use crate::types::hasura::*;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/graphql/schema.json",
    query_path = "src/graphql/get_upload_url.graphql",
    response_derives = "Debug,Clone,Deserialize,Serialize"
)]
pub struct GetUploadUrl;

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
    let upload_variables = get_upload_url::Variables {
        name: "election-event.json".to_string(),
        media_type: "application/json".to_string(),
        size: template_bytes.len() as i64,
        is_public: false,
        is_local: Some(false),
        election_event_id: None,
    };
    let upload_data = client
        .data_or_bail::<GetUploadUrl>(upload_variables)
        .await
        .context("failed to get an upload URL for the election-event template")?;
    let upload = upload_data
        .get_upload_url
        .ok_or_else(|| anyhow::anyhow!("get_upload_url returned no data"))?;

    let put_response = http
        .put(&upload.url)
        .header("Content-Type", "application/json")
        .body(template_bytes.to_vec())
        .send()
        .await
        .context("failed to upload the election-event template")?;
    if !put_response.status().is_success() {
        let status = put_response.status();
        let body = put_response.text().await.unwrap_or_default();
        bail!("upload of the election-event template failed (HTTP {status}): {body}");
    }

    let import_variables = import_election_event::Variables {
        tenant_id: tenant_id.to_string(),
        document_id: upload.document_id,
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
        bail!("election-event import rejected: {error}");
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
