// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::postgres::area::export_areas;
use crate::postgres::area_contest::export_area_contests;
use crate::postgres::candidate::export_candidates;
use crate::postgres::contest::export_contests;
use crate::postgres::election::export_elections;
use crate::postgres::election_event::export_election_event;
use crate::services::database::get_hasura_pool;
use crate::services::import_election_event::ImportElectionEventSchema;
use anyhow::{anyhow, Result};
use deadpool_postgres::{Client as DbClient, Transaction};
use futures::executor::block_on;
use futures::try_join;

pub async fn process_export(
    tenant_id: &str,
    election_event_id: &str,
) -> Result<ImportElectionEventSchema> {
    let mut hasura_db_client: DbClient = get_hasura_pool()
        .await
        .get()
        .await
        .map_err(|err| anyhow!("Error getting hasura db pool: {err}"))?;

    let hasura_transaction = hasura_db_client
        .transaction()
        .await
        .map_err(|err| anyhow!("Error starting hasura transaction: {err}"))?;

    let (election_event, elections, contests, candidates, areas, area_contests) = try_join!(
        export_election_event(&hasura_transaction, tenant_id, election_event_id),
        export_elections(&hasura_transaction, tenant_id, election_event_id),
        export_contests(&hasura_transaction, tenant_id, election_event_id),
        export_candidates(&hasura_transaction, tenant_id, election_event_id),
        export_areas(&hasura_transaction, tenant_id, election_event_id),
        export_area_contests(&hasura_transaction, tenant_id, election_event_id),
    )?;

    Err(anyhow!("not implemented"))
}
