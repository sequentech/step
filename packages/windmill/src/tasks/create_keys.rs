// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::postgres::election_event::get_election_event_by_id;
use crate::postgres::election_event::update_election_event_status;
use crate::postgres::keys_ceremony::get_keys_ceremony_by_id;
use crate::services::ceremonies::keys_ceremony::get_keys_ceremony_board;
use crate::services::database::get_hasura_pool;
use crate::services::election_event_board::get_election_event_board;
use crate::services::public_keys;
use crate::types::error::{Error, Result};
use anyhow::{anyhow, Context, Result as AnyhowResult};
use celery::error::TaskError;
use deadpool_postgres::Client as DbClient;
use sequent_core::ballot::ElectionEventStatus;
use sequent_core::ballot::VotingStatus;
use sequent_core::serialization::deserialize_with_path::*;
use sequent_core::services::keycloak;
use serde::{Deserialize, Serialize};
use std::default::Default;
use tracing::instrument;

#[derive(Deserialize, Debug, Serialize, Clone)]
pub struct CreateKeysBody {
    pub trustee_pks: Vec<String>,
    pub threshold: usize,
}

pub async fn create_keys_impl(
    body: CreateKeysBody,
    tenant_id: String,
    election_event_id: String,
    keys_ceremony_id: String,
) -> AnyhowResult<()> {
    let mut hasura_db_client: DbClient = get_hasura_pool().await.get().await?;

    let hasura_transaction = hasura_db_client.transaction().await?;

    let keys_ceremony = get_keys_ceremony_by_id(
        &hasura_transaction,
        &tenant_id,
        &election_event_id,
        &keys_ceremony_id,
    )
    .await
    .with_context(|| "error finding keys ceremony")?;

    let board_name = get_keys_ceremony_board(
        &hasura_transaction,
        &tenant_id,
        &election_event_id,
        &keys_ceremony,
    )
    .await?;
    // fetch election_event
    let election_event =
        get_election_event_by_id(&hasura_transaction, &tenant_id, &election_event_id).await?;

    // check config is not already created
    if keys_ceremony.is_default() {
        let status: Option<ElectionEventStatus> = match election_event.status.clone() {
            Some(value) => deserialize_value(value)?,
            None => None,
        };
        if status.map(|val| val.is_config_created()).unwrap_or(false) {
            return Err(anyhow!("bulletin board config already created"));
        }
    }

    // create config/keys for board
    public_keys::create_keys(
        board_name.as_str(),
        body.trustee_pks.clone(),
        body.threshold.clone(),
    )
    .await?;

    // update election event with status: keys created
    if keys_ceremony.is_default() {
        let mut new_status: ElectionEventStatus = Default::default();
        new_status.config_created = Some(true);
        let new_status_js = serde_json::to_value(new_status)?;

        update_election_event_status(
            &hasura_transaction,
            &tenant_id,
            &election_event_id,
            new_status_js,
        )
        .await?;
    }

    hasura_transaction
        .commit()
        .await
        .with_context(|| "error comitting transaction")?;

    Ok(())
}

#[instrument(err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task]
pub async fn create_keys(
    body: CreateKeysBody,
    tenant_id: String,
    election_event_id: String,
    keys_ceremony_id: String,
) -> Result<()> {
    create_keys_impl(body, tenant_id, election_event_id, keys_ceremony_id)
        .await
        .map_err(|err| Error::from(err.context("Task failed")))
}
