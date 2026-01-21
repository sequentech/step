// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::postgres::{keys_ceremony, trustee};
use crate::services::ceremonies::keys_ceremony::get_keys_ceremony_board;
use crate::services::ceremonies::serialize_logs::generate_logs;
use crate::services::ceremonies::serialize_logs::sort_logs;
use crate::services::database::get_hasura_pool;
use crate::services::election_event_board::get_election_event_board;
use crate::services::protocol_manager;
use crate::services::public_keys;
use crate::types::error::Result;
use anyhow::{anyhow, Context, Result as AnyhowResult};
use b3::messages::message::Message;
use b3::messages::statement::StatementType;
use celery::error::TaskError;
use deadpool_postgres::{Client as DbClient, Transaction};
use sequent_core::services::date::{get_now_utc_unix_ms, ISO8601};
use sequent_core::services::keycloak;
use sequent_core::types::ceremonies::{
    CeremoniesPolicy, KeysCeremonyExecutionStatus, KeysCeremonyStatus, Trustee as BasicTrustee,
    TrusteeStatus,
};
use sequent_core::types::hasura::core::Trustee;
use serde_json::Value;
use std::collections::HashSet;
use strand::signature::StrandSignaturePk;
use tracing::{event, info, instrument, Level};

#[instrument(skip(trustees_hasura, messages), err)]
fn get_trustee_status(
    trustee_name: &str,
    trustees_hasura: &Vec<Trustee>,
    messages: &Vec<Message>,
) -> Result<TrusteeStatus> {
    let Some(found_trustee) = trustees_hasura
        .iter()
        .find(|trustee| trustee.name == Some(trustee_name.to_string()))
    else {
        return Ok(TrusteeStatus::WAITING);
    };
    let Some(pk_str) = found_trustee.public_key.clone() else {
        return Ok(TrusteeStatus::WAITING);
    };
    let pk = StrandSignaturePk::from_der_b64_string(&pk_str)?;

    let valid_statements = vec![StatementType::PublicKey, StatementType::PublicKeySigned];

    let found_message = messages.iter().find(|message| {
        valid_statements.contains(&message.statement.get_kind()) && message.sender.pk == pk
    });

    if found_message.is_some() {
        Ok(TrusteeStatus::KEY_GENERATED)
    } else {
        Ok(TrusteeStatus::WAITING)
    }
}

pub async fn set_public_key_impl(
    tenant_id: String,
    election_event_id: String,
    keys_ceremony_id: String,
) -> AnyhowResult<()> {
    let mut hasura_db_client: DbClient = get_hasura_pool().await.get().await?;

    let hasura_transaction = hasura_db_client.transaction().await?;
    let keys_ceremony = keys_ceremony::get_keys_ceremony_by_id(
        &hasura_transaction,
        &tenant_id,
        &election_event_id,
        &keys_ceremony_id,
    )
    .await?;
    let current_status = keys_ceremony.status()?;
    if current_status.public_key.is_some() {
        info!("Public key already set");
        return Ok(());
    }
    let execution_status = keys_ceremony.execution_status()?;
    if execution_status != KeysCeremonyExecutionStatus::IN_PROGRESS {
        info!("Unexpected status {}", execution_status);
        return Ok(());
    }
    let (board_name, _) = get_keys_ceremony_board(
        &hasura_transaction,
        &tenant_id,
        &election_event_id,
        &keys_ceremony,
    )
    .await?;
    let public_key_opt = public_keys::get_public_key(board_name.clone()).await.ok();
    // verify trustee names and fetch their objects to get their ids
    let trustee_names = current_status
        .trustees
        .clone()
        .into_iter()
        .map(|trustee| trustee.name)
        .collect::<HashSet<String>>();
    let trustees_by_name = trustee::get_trustees_by_name(
        &hasura_transaction,
        &tenant_id,
        &trustee_names.clone().into_iter().collect::<Vec<_>>(),
    )
    .await?;

    let trustees_by_name_names = trustees_by_name
        .clone()
        .into_iter()
        .filter_map(|trustee| trustee.name)
        .collect::<HashSet<String>>();
    // we should have a list with the same trustees
    if trustee_names != trustees_by_name_names {
        return Err(anyhow!(
            "trustee_names don't correspond to trustees_by_name"
        ));
    }

    // Timestamp since last update.
    let next_timestamp = keys_ceremony
        .last_updated_at
        .with_context(|| "empty last_updated_at")?
        .timestamp() as u64;

    let messages = protocol_manager::get_board_public_key_messages(&board_name).await?;
    let mut new_logs = generate_logs(&messages, next_timestamp, &vec![0])?;
    let mut logs = current_status.logs.clone();
    logs.append(&mut new_logs);

    let keys_ceremony_policy = keys_ceremony.policy().clone();

    // if we have a public key, and the policy is automated, we can set the status to success
    let new_execution_status = match (keys_ceremony_policy.clone(), public_key_opt.clone()) {
        (CeremoniesPolicy::AUTOMATED_CEREMONIES, Some(_)) => KeysCeremonyExecutionStatus::SUCCESS,
        _ => KeysCeremonyExecutionStatus::IN_PROGRESS,
    };

    let new_status: Value = serde_json::to_value(KeysCeremonyStatus {
        stop_date: Some(get_now_utc_unix_ms().to_string()),
        public_key: public_key_opt.clone(),
        logs: sort_logs(&logs),
        trustees: current_status
            .trustees
            .clone()
            .into_iter()
            .map(|trustee| -> Result<BasicTrustee> {
                Ok(BasicTrustee {
                    name: trustee.name.clone(),
                    status: get_trustee_status(&trustee.name, &trustees_by_name, &messages)?,
                })
            })
            .collect::<Result<Vec<BasicTrustee>>>()?,
    })?;

    // update public key
    keys_ceremony::update_keys_ceremony_status(
        &hasura_transaction,
        &tenant_id,
        &election_event_id,
        &keys_ceremony_id,
        &new_status,
        &new_execution_status.to_string(),
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
#[celery::task(max_retries = 0)]
pub async fn set_public_key(
    tenant_id: String,
    election_event_id: String,
    keys_ceremony_id: String,
) -> Result<()> {
    set_public_key_impl(tenant_id, election_event_id, keys_ceremony_id).await?;

    Ok(())
}
