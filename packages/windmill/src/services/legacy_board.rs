// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The client of the old core's b4 board (`packages/b4`, Postgres backed) and
//! the name of a keys ceremony's board on it, which the bulletin board export,
//! import and deletion still use. The keys ceremony and the tally run on the
//! new core (`crate::services::protocol_board`); this module goes away with
//! `packages/b4`.

use anyhow::{anyhow, Context, Result};
use b4::client::pgsql::{PgsqlB3Client, PgsqlConnectionParams};
use deadpool_postgres::Transaction;
use sequent_core::types::hasura::core::KeysCeremony;
use std::env;
use tracing::instrument;

use crate::postgres::election::get_elections_by_keys_ceremony_id;
use crate::postgres::election_event::get_election_event_by_id;
use crate::services::election_event_board::get_election_event_board;
use crate::services::electoral_log_board::get_election_board;

#[instrument(err)]
pub async fn get_b3_pgsql_client() -> Result<PgsqlB3Client> {
    let username = env::var("B4_PG_USER").context("B4_PG_USER must be set")?;
    let password = env::var("B4_PG_PASSWORD").context("B4_PG_PASSWORD must be set")?;
    let host = env::var("B4_PG_HOST").context("B4_PG_HOST must be set")?;
    let port = env::var("B4_PG_PORT").context("B4_PG_PORT must be set")?;
    let database = env::var("B4_PG_DATABASE").context("B4_PG_DATABASE must be set")?;

    let port: u32 = port.parse::<u32>()?;

    let c = PgsqlConnectionParams::new(&host, port, &username, &password);
    let c_db = c.with_database(&database);
    let client = PgsqlB3Client::new(&c_db).await?;

    Ok(client)
}

// returns (board_name, election_id), where the election_id might be None for an event Board
#[instrument(skip(transaction), err)]
pub async fn get_keys_ceremony_board(
    transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    keys_ceremony: &KeysCeremony,
) -> Result<(String, Option<String>)> {
    if keys_ceremony.is_default() {
        // fetch election_event
        let election_event =
            get_election_event_by_id(transaction, tenant_id, election_event_id).await?;

        // get board name
        let board_name = get_election_event_board(election_event.bulletin_board_reference.clone())
            .with_context(|| "missing bulletin board")?;
        Ok((board_name, None))
    } else {
        let election = get_elections_by_keys_ceremony_id(
            transaction,
            tenant_id,
            election_event_id,
            &keys_ceremony.id,
        )
        .await?
        .into_iter()
        .next()
        .ok_or_else(|| {
            anyhow!(
                "Can't find election with keys ceremony {}",
                keys_ceremony.id
            )
        })?;
        let slug = std::env::var("ENV_SLUG").with_context(|| "missing env var ENV_SLUG")?;
        let board = get_election_board(tenant_id, &election.id, &slug);
        Ok((board, Some(election.id)))
    }
}
