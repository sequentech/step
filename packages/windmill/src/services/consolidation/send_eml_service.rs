// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::{anyhow, Context, Result};
use deadpool_postgres::Client as DbClient;
use deadpool_postgres::Transaction;

use crate::postgres::election_event::get_election_event_by_id;
use crate::services::database::get_hasura_pool;

pub async fn send_eml_service(
    tenant_id: String,
    election_event_id: String,
    tally_session_id: String,
) -> Result<()> {
    let mut hasura_db_client: DbClient = get_hasura_pool()
        .await
        .get()
        .await
        .with_context(|| "Error acquiring hasura connection pool")?;
    let hasura_transaction = hasura_db_client
        .transaction()
        .await
        .with_context(|| "Error acquiring hasura transaction")?;
    let election_event =
        get_election_event_by_id(&hasura_transaction, &tenant_id, &election_event_id)
            .await
            .with_context(|| "Error fetching election event")?;

    hasura_transaction
        .commit()
        .await
        .with_context(|| "error comitting transaction")?;
    Ok(())
}
