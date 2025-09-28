// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::postgres::election_event::get_election_event_by_id;
use crate::postgres::keys_ceremony::{get_keys_ceremony_by_id, update_keys_ceremony_status};
use crate::postgres::trustee::get_trustees_by_id;
use crate::services::ceremonies::keys_ceremony::get_keys_ceremony_board;
use crate::services::database::get_hasura_pool;
use crate::services::protocol_manager::check_configuration_exists;
use crate::services::{ceremonies, public_keys};
use crate::types::error::{Error, Result};
use anyhow::{anyhow, Context, Result as AnyhowResult};
use celery::error::TaskError;
use deadpool_postgres::{Client as DbClient, Transaction};
use sequent_core::types::ceremonies::{
    CeremoniesPolicy, KeysCeremonyExecutionStatus, KeysCeremonyStatus, Trustee, TrusteeStatus,
};
use sequent_core::types::hasura::core::KeysCeremony;
use serde::{Deserialize, Serialize};
use std::default::Default;
use tracing::{info, instrument};

#[derive(Deserialize, Debug, Serialize, Clone)]
pub struct CreateKeysBody {
    pub trustee_pks: Vec<String>,
    pub threshold: usize,
}

pub async fn create_keys_impl(
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

    let trustees =
        get_trustees_by_id(&hasura_transaction, &tenant_id, &keys_ceremony.trustee_ids).await?;
    info!("trustees: {:?}", trustees);
    let trustee_pks = trustees
        .clone()
        .into_iter()
        .filter_map(|trustee| trustee.public_key)
        .collect();
    info!("trustee_pks: {:?}", trustee_pks);

    let (board_name, _) = get_keys_ceremony_board(
        &hasura_transaction,
        &tenant_id,
        &election_event_id,
        &keys_ceremony,
    )
    .await?;

    let execution_status = keys_ceremony.execution_status()?;
    let status = keys_ceremony.status()?;

    // check config is not already created
    if execution_status != KeysCeremonyExecutionStatus::STARTED || status.public_key.is_some() {
        info!("Unexpected status: {}", execution_status);
        return Ok(());
    }

    let configuration_exists = check_configuration_exists(board_name.as_str()).await?;

    if !configuration_exists {
        // create config/keys for board
        public_keys::create_keys(
            &hasura_transaction,
            &tenant_id,
            &election_event_id,
            board_name.as_str(),
            trustee_pks,
            keys_ceremony.threshold as usize,
        )
        .await?;
    }

    update_keys_ceremony_status(
        &hasura_transaction,
        &tenant_id,
        &election_event_id,
        &keys_ceremony.id,
        &serde_json::to_value(status)?,
        &KeysCeremonyExecutionStatus::IN_PROGRESS.to_string(),
    )
    .await?;

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
    tenant_id: String,
    election_event_id: String,
    keys_ceremony_id: String,
) -> Result<()> {
    create_keys_impl(tenant_id, election_event_id, keys_ceremony_id)
        .await
        .map_err(|err| Error::from(err.context("Task failed")))
}
