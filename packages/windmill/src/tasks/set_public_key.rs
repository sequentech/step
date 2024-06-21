// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::Context;
use board_messages::braid::message::Message;
use board_messages::braid::statement::StatementType;
use celery::error::TaskError;
use sequent_core::services::keycloak;
use sequent_core::types::ceremonies::{CeremonyStatus, ExecutionStatus, Trustee, TrusteeStatus};
use serde_json::Value;
use std::collections::HashSet;
use strand::signature::StrandSignaturePk;
use tracing::{event, instrument, Level};

use crate::hasura;
use crate::hasura::trustee::get_trustees_by_name;
use crate::services::ceremonies::keys_ceremony::get_keys_ceremony_status;
use crate::services::ceremonies::serialize_logs::generate_logs;
use crate::services::ceremonies::serialize_logs::sort_logs;
use crate::services::date::{get_now_utc_unix_ms, ISO8601};
use crate::services::election_event_board::get_election_event_board;
use crate::services::protocol_manager;
use crate::services::public_keys;
use crate::tasks::set_public_key::get_trustees_by_name::GetTrusteesByNameSequentBackendTrustee;
use crate::types::error::Result;

#[instrument(skip(trustees_hasura, messages), err)]
fn get_trustee_status(
    trustee_name: &str,
    trustees_hasura: &Vec<GetTrusteesByNameSequentBackendTrustee>,
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

#[instrument(err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task(max_retries = 0)]
pub async fn set_public_key(tenant_id: String, election_event_id: String) -> Result<()> {
    let auth_headers = keycloak::get_client_credentials().await?;
    let election_event_response = hasura::election_event::get_election_event(
        auth_headers.clone(),
        tenant_id.clone(),
        election_event_id.clone(),
    )
    .await?
    .data
    .with_context(|| "can't find election event")?;

    let election_event = &election_event_response.sequent_backend_election_event[0];

    if election_event.public_key.is_some() {
        event!(Level::INFO, "Public key already set");
    }

    let bulletin_board_reference = election_event.bulletin_board_reference.clone();
    let board_name = match get_election_event_board(bulletin_board_reference) {
        Some(board_name) => board_name,
        None => {
            event!(Level::INFO, "Public key not found");
            return Ok(());
        }
    };

    // set public key in the election event
    let public_key_opt = public_keys::get_public_key(board_name.clone()).await.ok();
    if let Some(public_key) = public_key_opt.clone() {
        hasura::election_event::update_election_event_public_key(
            auth_headers.clone(),
            tenant_id.clone(),
            election_event_id.clone(),
            public_key.clone(),
        )
        .await?;
    }

    // find the keys ceremony, and then update it
    let keys_ceremonies = hasura::keys_ceremony::get_keys_ceremonies(
        auth_headers.clone(),
        tenant_id.clone(),
        election_event_id.clone(),
    )
    .await?
    .data
    .with_context(|| "error listing existing keys ceremonies")?
    .sequent_backend_keys_ceremony;

    if keys_ceremonies.len() == 0 {
        event!(Level::INFO, "Strange, no ceremonies!");
        return Ok(());
    }

    if keys_ceremonies.len() > 1 {
        event!(
            Level::ERROR,
            "Strange, too many ceremonies! we'll just update the first one"
        );
    }
    let keys_ceremony = &keys_ceremonies[0];
    if let Some(execution_status) = keys_ceremony.execution_status.clone() {
        if execution_status != ExecutionStatus::NOT_STARTED.to_string()
            && execution_status != ExecutionStatus::IN_PROCESS.to_string()
        {
            event!(
                Level::ERROR,
                "Strange, keys ceremony in wrong execution_status={:?}",
                keys_ceremony.execution_status
            );
            return Err("keys ceremony in wrong execution_status".into());
        }
    }
    let current_status: CeremonyStatus = get_keys_ceremony_status(keys_ceremony.status.clone())?;

    // verify trustee names and fetch their objects to get their ids
    let trustee_names = current_status
        .trustees
        .clone()
        .into_iter()
        .map(|trustee| trustee.name)
        .collect::<HashSet<String>>();
    let trustees_by_name = get_trustees_by_name(
        &auth_headers,
        &tenant_id,
        &trustee_names.clone().into_iter().collect::<Vec<_>>(),
    )
    .await?
    .data
    .with_context(|| "can't find trustees")?
    .sequent_backend_trustee;

    let trustees_by_name_names = trustees_by_name
        .clone()
        .into_iter()
        .filter_map(|trustee| trustee.name)
        .collect::<HashSet<String>>();
    // we should have a list with the same trustees
    if trustee_names != trustees_by_name_names {
        return Err("trustee_names don't correspond to trustees_by_name".into());
    }

    // Timestamp since last update.
    let next_timestamp = ISO8601::to_date(&keys_ceremony.last_updated_at)?.timestamp() as u64;

    let messages = protocol_manager::get_board_public_key_messages(&board_name).await?;
    let mut new_logs = generate_logs(&messages, next_timestamp, &vec![0])?;
    let mut logs = current_status.logs.clone();
    logs.append(&mut new_logs);

    let new_execution_status: String = ExecutionStatus::IN_PROCESS.to_string();
    let new_status: Value = serde_json::to_value(CeremonyStatus {
        stop_date: Some(get_now_utc_unix_ms().to_string()),
        public_key: public_key_opt.clone(),
        logs: sort_logs(&logs),
        trustees: current_status
            .trustees
            .clone()
            .into_iter()
            .map(|trustee| -> Result<Trustee> {
                Ok(Trustee {
                    name: trustee.name.clone(),
                    status: get_trustee_status(&trustee.name, &trustees_by_name, &messages)?,
                })
            })
            .collect::<Result<Vec<Trustee>>>()?,
    })?;

    // update public key
    hasura::keys_ceremony::update_keys_ceremony_status(
        auth_headers.clone(),
        tenant_id.clone(),
        election_event_id.clone(),
        keys_ceremony.id.clone(),
        new_status,
        new_execution_status,
    )
    .await?;

    Ok(())
}
