// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::Result;
use celery::error::TaskError;
use celery::prelude::*;
use immu_board::{BoardClient, util::get_board_name};
use sequent_core;
use sequent_core::services::{openid, connection};
use serde::{Deserialize, Serialize};
use std::env;
use tracing::instrument;

use crate::hasura;
use crate::services::election_event_board::BoardSerializable;
use crate::hasura::election_event::{
    insert_election_event::sequent_backend_election_event_insert_input as InsertElectionEventInput
};

async fn get_board_client() -> Result<BoardClient> {
    let username = env::var("IMMUDB_USER")
        .expect(&format!("IMMUDB_USER must be set"));
    let password = env::var("IMMUDB_PASSWORD")
        .expect(&format!("IMMUDB_PASSWORD must be set"));
    let server_url = env::var("IMMUDB_SERVER_URL")
        .expect(&format!("IMMUDB_SERVER_URL must be set"));
    let mut client = BoardClient::new(&server_url, &username, &password).await?;
    client.login(&username, &password).await?;
    Ok(client)
}

#[instrument]
pub async fn create_immu_board(
    auth_headers: &connection::AuthHeaders,
    tenant_id: &str,
    election_event_id: &str,
) -> TaskResult<()> {

    let index_db = env::var("IMMUDB_INDEX_DB").expect(&format!("IMMUDB_INDEX_DB must be set"));
    let board_name = get_board_name(tenant_id, election_event_id);
    let mut board_client = get_board_client()
        .await
        .map_err(|err| TaskError::UnexpectedError(format!("{:?}", err)))?;
    let board = board_client
        .create_board(&index_db, &board_name)
        .await
        .map_err(|err| TaskError::UnexpectedError(format!("{:?}", err)))?;
    Ok(())
}

#[instrument]
pub async fn create_keycloak_realm(
    auth_headers: &connection::AuthHeaders,
    object: &InsertElectionEventInput,
) -> TaskResult<()> {
    todo!()
}

#[instrument]
pub async fn insert_election_event_db(
    auth_headers: &connection::AuthHeaders,
    object: &InsertElectionEventInput,
) -> TaskResult<()> {
    let hasura_response = hasura::election_event::insert_election_event_f(
        auth_headers.clone(),
        object.clone(),
    )
    .await
    .map_err(|err| TaskError::UnexpectedError(format!("{:?}", err)))?;

    Ok(())
}

#[instrument(skip_all)]
#[celery::task]
pub async fn insert_election_event_t(
    object: InsertElectionEventInput,
) -> TaskResult<()> {
    let auth_headers = openid::get_client_credentials()
        .await
        .map_err(|err| TaskError::UnexpectedError(format!("{:?}", err)))?;

    create_immu_board(
        &auth_headers,
        &object.tenant_id
            .as_ref()
            .ok_or("empty-tenant-id")
            .map_err(|err| TaskError::UnexpectedError(format!("{:?}", err)))?,
        &object.id
            .as_ref()
            .ok_or("empty-tenant-id")
            .map_err(|err| TaskError::UnexpectedError(format!("{:?}", err)))?,
    ).await?;
    create_keycloak_realm(&auth_headers, &object).await?;
    insert_election_event_db(&auth_headers, &object).await?;

    Ok(())
}
