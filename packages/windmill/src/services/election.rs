// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::{anyhow, Context, Result};
use deadpool_postgres::Client as DbClient;
use deadpool_postgres::Transaction;
use sequent_core::services::translations::{Alias, Name};
use sequent_core::types::hasura::core::Election;
use sequent_core::types::keycloak::{User, VotesInfo};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio_postgres::row::Row;
use tracing::{info, instrument};
use uuid::Uuid;

use crate::postgres::election::get_elections;

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct ElectionHead {
    pub id: String,
    pub name: String,
    pub alias: Option<String>,
    pub external_id: Option<String>,
}

impl TryFrom<Election> for ElectionHead {
    type Error = anyhow::Error;
    fn try_from(item: Election) -> Result<Self> {
        let default_language = item.get_default_language();
        let election = item.clone();
        Ok(ElectionHead {
            id: election.id.clone(),
            name: election.get_name(&default_language),
            alias: election.get_alias(&default_language),
            external_id: election.external_id,
        })
    }
}

#[instrument(err)]
pub async fn get_election_event_elections(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
) -> Result<Vec<ElectionHead>> {
    let election_event_elections = get_elections(hasura_transaction, tenant_id, election_event_id)
        .await
        .with_context(|| "Error get election event elections")?;

    let elections = election_event_elections
        .into_iter()
        .map(|row| -> Result<ElectionHead> { row.try_into() })
        .collect::<Result<Vec<ElectionHead>>>()
        .with_context(|| "Error collecting the elections")?;

    Ok(elections)
}
