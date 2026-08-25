// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Phase 1 — provisioning: create a tenant, import an election event into
//! it, publish, open voting, and provision voters. See
//! `LOAD_TEST_DESIGN.md` § Phase 1.

mod import;
mod publish;
mod tasks;
mod voters;
mod voting_status;

pub use import::import_election_event;
pub use publish::publish;
pub use voters::{provision_voter, provision_voters, voter_credential, VoterCredential};
pub use voting_status::open_voting;

use anyhow::{Context, Result};
use graphql_client::GraphQLQuery;

use crate::hasura::HasuraClient;
use crate::types::hasura::*;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/graphql/schema.json",
    query_path = "src/graphql/insert_tenant.graphql",
    response_derives = "Debug,Clone,Deserialize,Serialize"
)]
pub struct InsertTenant;

pub struct CreatedTenant {
    pub id: String,
    pub slug: String,
}

/// Creates a tenant and its Keycloak realm. `insertTenant` only *enqueues*
/// realm creation — the id it returns is generated and handed back
/// immediately, before the realm actually exists
/// (`packages/harvest/src/routes/insert_tenant.rs:71-79`) — so this polls
/// the `task_execution` it comes with before returning.
pub async fn create_tenant(client: &HasuraClient, slug: &str) -> Result<CreatedTenant> {
    let variables = insert_tenant::Variables {
        slug: slug.to_string(),
    };
    let data = client
        .data_or_bail::<InsertTenant>(variables)
        .await
        .with_context(|| format!("failed to create tenant `{slug}`"))?;
    let created = data
        .insert_tenant
        .ok_or_else(|| anyhow::anyhow!("insertTenant returned no data for `{slug}`"))?;
    if let Some(error_msg) = created.error_msg {
        anyhow::bail!("insertTenant rejected `{slug}`: {error_msg}");
    }
    if let Some(task_execution) = created.task_execution {
        tasks::poll_task_execution(client, &task_execution.id)
            .await
            .with_context(|| {
                format!(
                    "tenant `{slug}` (id {}) creation task did not complete",
                    created.id
                )
            })?;
    }
    Ok(CreatedTenant {
        id: created.id,
        slug: created.slug,
    })
}

pub struct ProvisionedElectionEvent {
    pub tenant_id: String,
    pub election_event_id: String,
    pub area_id: String,
    pub election_ids: Vec<String>,
    pub voters: Vec<VoterCredential>,
}

/// Runs every Phase 1 step for one election event, in order: import,
/// publish, open voting, then provision `voter_count` voters.
pub async fn provision_election_event(
    client: &HasuraClient,
    http: &reqwest::Client,
    tenant_id: &str,
    template_bytes: &[u8],
    voter_count: u32,
) -> Result<ProvisionedElectionEvent> {
    let election_event_id = import_election_event(client, http, tenant_id, template_bytes).await?;
    publish(client, &election_event_id).await?;
    open_voting(client, &election_event_id).await?;

    let area_ids = voters::get_area_ids(client, &election_event_id).await?;
    let area_id = area_ids.into_iter().next().ok_or_else(|| {
        anyhow::anyhow!("election event {election_event_id} has no areas to assign voters to")
    })?;
    let election_ids = voters::get_election_ids(client, &election_event_id).await?;

    let voters = voters::provision_voters(
        client,
        tenant_id,
        &election_event_id,
        &area_id,
        &election_ids,
        voter_count,
    )
    .await?;

    Ok(ProvisionedElectionEvent {
        tenant_id: tenant_id.to_string(),
        election_event_id,
        area_id,
        election_ids,
        voters,
    })
}
