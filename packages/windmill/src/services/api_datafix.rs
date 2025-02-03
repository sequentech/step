// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::postgres::election::{get_election_by_id, get_elections, update_election_voting_status};
use crate::postgres::election_event::{
    get_all_tenant_election_events, get_election_event_by_id, update_election_event_status,
};
use crate::services::providers::transactions_provider::provide_hasura_transaction;
use anyhow::{anyhow, Result};
use deadpool_postgres::Transaction;
use sequent_core::serialization::deserialize_with_path::deserialize_value;
use sequent_core::types::hasura::core::ElectionEvent;
use tracing::{info, instrument};

#[instrument(err)]
pub async fn get_datafix_election_event_id(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    datafix_id: &str,
) -> Result<String> {
    let election_events = get_all_tenant_election_events(hasura_transaction, tenant_id)
        .await
        .map_err(|err| anyhow!("Error getting election events: {err}"))?;

    // let election_events: Vec<ElectionEvent>
    info!("election_events: {:?}", election_events);
    // WIP
    Ok("".to_string())
}
