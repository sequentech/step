// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::services::database::get_hasura_pool;
use crate::types::error::{Error, Result};
use anyhow::{anyhow, Context};
use deadpool_postgres::Client as DbClient;
use deadpool_postgres::Transaction;
use tracing::{event, info, instrument, Level};

#[instrument(err)]
pub async fn handle_tally_session_error(
    error: &Error,
    tenant_id: &str,
    election_event_id: &str,
    tally_session_id: &str,
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

    hasura_transaction
        .commit()
        .await
        .with_context(|| "error comitting transaction")?;

    Ok(())
}
