// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use celery::error::TaskError;
use immu_board::util::get_board_name;
use sequent_core;
use sequent_core::services::{connection, keycloak};

use std::env;
use tracing::instrument;

use crate::hasura;
use crate::hasura::election_event::insert_election_event::sequent_backend_election_event_insert_input as InsertElectionEventInput;
use crate::services::protocol_manager::get_board_client;
use crate::types::error::Result;

#[instrument]
pub async fn create_immu_board(
    auth_headers: &connection::AuthHeaders,
    tenant_id: &str,
    election_event_id: &str,
) -> Result<()> {
    let index_db = env::var("IMMUDB_INDEX_DB").expect(&format!("IMMUDB_INDEX_DB must be set"));
    let board_name = get_board_name(tenant_id, election_event_id);
    let mut board_client = get_board_client().await?;
    let _board = board_client.create_board(&index_db, &board_name).await?;
    Ok(())
}

#[instrument]
pub async fn create_keycloak_realm(
    auth_headers: &connection::AuthHeaders,
    object: &InsertElectionEventInput,
) -> Result<()> {
    todo!()
}

#[instrument]
pub async fn insert_election_event_db(
    auth_headers: &connection::AuthHeaders,
    object: &InsertElectionEventInput,
) -> Result<()> {
    let _hasura_response =
        hasura::election_event::insert_election_event(auth_headers.clone(), object.clone()).await?;

    Ok(())
}

#[instrument(skip_all)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task]
pub async fn insert_election_event_t(object: InsertElectionEventInput) -> Result<()> {
    let auth_headers = keycloak::get_client_credentials().await?;

    create_immu_board(
        &auth_headers,
        &object.tenant_id.as_ref().ok_or("empty-tenant-id")?,
        &object.id.as_ref().ok_or("empty-election-event-id")?,
    )
    .await?;
    create_keycloak_realm(&auth_headers, &object).await?;
    insert_election_event_db(&auth_headers, &object).await?;

    Ok(())
}
