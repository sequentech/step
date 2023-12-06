// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::hasura::keys_ceremony::get_keys_ceremony;
use anyhow::{anyhow, Context, Result};
use sequent_core::services::connection;
use sequent_core::services::keycloak;
use sequent_core::types::ceremonies::ExecutionStatus;

pub async fn find_keys_ceremony(
    auth_headers: connection::AuthHeaders,
    tenant_id: String,
    election_event_id: String,
) -> Result<String> {
    // find if there's any previous ceremony. There should be one and it should
    // have finished successfully.
    let keys_ceremonies = get_keys_ceremony(
        auth_headers.clone(),
        tenant_id.clone(),
        election_event_id.clone(),
    )
    .await?
    .data
    .with_context(|| "error listing existing keys ceremonies")?
    .sequent_backend_keys_ceremony;

    let successful_ceremonies: Vec<_> = keys_ceremonies
        .into_iter()
        .filter(|ceremony| {
            ceremony
                .execution_status
                .clone()
                .map(|value| value == ExecutionStatus::SUCCESS.to_string())
                .unwrap_or(false)
        })
        .collect();
    if 0 == successful_ceremonies.len() {
        return Err(anyhow!("Can't find keys ceremony"));
    }
    if successful_ceremonies.len() > 1 {
        return Err(anyhow!("Expected a single keys ceremony"));
    }
    Ok(successful_ceremonies[0].id.clone())
}

pub async fn create_tally_ceremony(
    tenant_id: String,
    election_event_id: String,
    election_ids: Vec<String>,
) -> Result<String> {
    let auth_headers = keycloak::get_client_credentials().await?;
    let keys_ceremony_id = find_keys_ceremony(auth_headers, tenant_id, election_event_id).await?;
    Ok(keys_ceremony_id)
}
