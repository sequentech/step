// SPDX-FileCopyrightText: 2025 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::postgres::election_event::get_election_event_by_id;
use crate::postgres::election_event::update_election_event_archived;
use crate::services::protocol_manager::get_event_board;
use crate::services::protocol_manager::get_immudb_client;
use anyhow::{anyhow, Context, Result};
use deadpool_postgres::Transaction;
use tracing::{event, info, instrument, Level};

#[instrument(err, skip_all)]
pub async fn archive_election_event(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
) -> Result<()> {
    let election_event =
        get_election_event_by_id(hasura_transaction, tenant_id, election_event_id).await?;

    let mut client = get_immudb_client().await?;
    let slug = std::env::var("ENV_SLUG").with_context(|| "missing env var ENV_SLUG")?;
    let board_name = get_event_board(tenant_id, election_event_id, &slug);

    event!(Level::INFO, "database name = {board_name}");

    let database_info = client
        .get_database(&board_name)
        .await
        .map_err(|err| anyhow!("error fetching immudb database info: {err:?}"))?;

    let Some(database_info) = database_info else {
        return Err(anyhow!("Error reading immudb database {board_name}"));
    };

    if election_event.is_archived {
        if !database_info.loaded {
            client
                .load_database(&board_name)
                .await
                .map_err(|err| anyhow!("error loading immudb database: {err:?}"))?;
        } else {
            info!("database already loaded");
        }
    } else {
        if database_info.loaded {
            client
                .unload_database(&board_name)
                .await
                .map_err(|err| anyhow!("error unloading immudb database: {err:?}"))?;
        } else {
            info!("database already unloaded");
        }
    }

    update_election_event_archived(
        hasura_transaction,
        tenant_id,
        election_event_id,
        !election_event.is_archived,
    )
    .await?;
    Ok(())
}
