// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::hasura::election_event::{get_election_event_helper, update_election_event_status};
use crate::hasura::keys_ceremony::get_keys_ceremonies;
use crate::hasura::keys_ceremony::{
    get_keys_ceremony_by_id, insert_keys_ceremony, update_keys_ceremony_status,
};
use crate::hasura::trustee::get_trustees_by_name;
use crate::services::celery_app::get_celery_app;
use crate::services::ceremonies::serialize_logs::*;
use crate::services::election_event_board::get_election_event_board;
use crate::services::election_event_status::get_election_event_status;
use crate::services::electoral_log::ElectoralLog;
use crate::services::private_keys::get_trustee_encrypted_private_key;
use crate::tasks::create_keys::{create_keys, CreateKeysBody};
use anyhow::{anyhow, Context, Result};
use sequent_core::serialization::deserialize_with_path::*;
use sequent_core::services::connection;
use sequent_core::services::jwt::JwtClaims;
use sequent_core::services::keycloak;
use sequent_core::types::ceremonies::{CeremonyStatus, ExecutionStatus, Trustee, TrusteeStatus};
use serde_json::from_value;
use serde_json::Value;
use tracing::instrument;
use tracing::{event, Level};
use uuid::Uuid;

#[instrument(err)]
pub fn get_keys_ceremony_status(input: Option<Value>) -> Result<CeremonyStatus> {
    input
        .map(|value| {
            deserialize_value(value)
                .map_err(|err| anyhow!("Error parsing keys ceremony status: {:?}", err))
        })
        .ok_or(anyhow!("Missing keys ceremony status"))
        .flatten()
}

#[instrument(err)]
pub async fn get_private_key(
    claims: JwtClaims,
    tenant_id: String,
    election_event_id: String,
    keys_ceremony_id: String,
) -> Result<String> {
    let auth_headers = keycloak::get_client_credentials().await?;
    let celery_app = get_celery_app().await;

    // The trustee name is simply the username of the user
    let trustee_name = claims
        .trustee
        .ok_or(anyhow!("trustee name not found"))?;

    // get the keys ceremonies for this election event
    let keys_ceremony = get_keys_ceremonies(
        auth_headers.clone(),
        tenant_id.clone(),
        election_event_id.clone(),
    )
    .await?
    .data
    .with_context(|| "error listing existing keys ceremonies")?
    .sequent_backend_keys_ceremony
    .into_iter()
    .find(|ceremony| ceremony.id == keys_ceremony_id)
    .with_context(|| "error listing existing keys ceremonies")?;
    // check keys_ceremony has correct execution status
    if keys_ceremony.execution_status != Some(ExecutionStatus::IN_PROCESS.to_string()) {
        return Err(anyhow!("Keys ceremony not in ExecutionStatus::IN_PROCESS"));
    }

    // get ceremony status
    let current_status: CeremonyStatus = deserialize_value(
        keys_ceremony
            .status
            .clone()
            .ok_or(anyhow!("Empty keys ceremony status"))?,
    )
    .with_context(|| "error parsing keys ceremony current status")?;

    // check the trustee is part of this ceremony
    if let None = current_status
        .trustees
        .clone()
        .into_iter()
        .find(|trustee| trustee.name == trustee_name)
    {
        return Err(anyhow!("Trustee not part of the keys ceremony"));
    }

    // fetch election_event
    let election_event = get_election_event_helper(
        auth_headers.clone(),
        tenant_id.clone(),
        election_event_id.clone(),
    )
    .await?;

    // get board name
    let board_name = get_election_event_board(election_event.bulletin_board_reference.clone())
        .with_context(|| "missing bulletin board")?;

    let trustee_public_key =
        get_trustees_by_name(&auth_headers, &tenant_id, &vec![trustee_name.clone()])
            .await
            .with_context(|| "can't find trustee in the database")?
            .data
            .with_context(|| "error fetching election event")?
            .sequent_backend_trustee[0]
            .public_key
            .clone()
            .ok_or(anyhow!("can't get election event"))?;

    // get the encrypted private key
    let encrypted_private_key =
        get_trustee_encrypted_private_key(board_name.as_str(), trustee_public_key.as_str()).await?;

    // Update ceremony with the information that this trustee did get the
    // private key
    let status: Value = serde_json::to_value(CeremonyStatus {
        stop_date: None,
        public_key: current_status.public_key.clone(),
        logs: append_keys_trustee_download_log(&current_status.logs, &trustee_name),
        trustees: current_status
            .trustees
            .clone()
            .into_iter()
            .map(|trustee| {
                if (trustee.name == trustee_name) {
                    Ok(Trustee {
                        name: trustee.name,
                        status: TrusteeStatus::KEY_RETRIEVED,
                    })
                } else {
                    Ok(trustee.clone())
                }
            })
            .collect::<Result<Vec<Trustee>>>()?,
    })?;

    // update keys-ceremony into the database using graphql
    update_keys_ceremony_status(
        auth_headers.clone(),
        tenant_id.clone(),
        election_event_id.clone(),
        keys_ceremony_id.clone(),
        /* status */ status,
        /* execution_status */
        keys_ceremony
            .execution_status
            .with_context(|| "empty current execution_status")?,
    )
    .await
    .with_context(|| "couldn't update keys ceremony")?;

    event!(
        Level::INFO,
        "Retrieved private key for electionEventId={}, keysCeremonyId={}, trusteeName={}",
        election_event_id.clone(),
        keys_ceremony_id.clone(),
        trustee_name.clone()
    );
    Ok(encrypted_private_key)
}

#[instrument(skip(auth_headers), err)]
pub async fn find_trustee_private_key(
    auth_headers: &connection::AuthHeaders,
    tenant_id: &str,
    election_event_id: &str,
    trustee_name: &str,
) -> Result<String> {
    // fetch election_event
    let election_event = get_election_event_helper(
        auth_headers.clone(),
        tenant_id.to_string(),
        election_event_id.to_string(),
    )
    .await?;

    // get board name
    let board_name = get_election_event_board(election_event.bulletin_board_reference.clone())
        .with_context(|| "missing bulletin board")?;

    let trustee_public_key =
        get_trustees_by_name(auth_headers, tenant_id, &vec![trustee_name.to_string()])
            .await
            .with_context(|| "can't find trustee in the database")?
            .data
            .with_context(|| "error fetching election event")?
            .sequent_backend_trustee[0]
            .public_key
            .clone()
            .ok_or(anyhow!("can't get election event"))?;

    // get the encrypted private key
    get_trustee_encrypted_private_key(board_name.as_str(), trustee_public_key.as_str()).await
}

#[instrument(err)]
pub async fn check_private_key(
    claims: JwtClaims,
    tenant_id: String,
    election_event_id: String,
    keys_ceremony_id: String,
    private_key_base64: String,
) -> Result<bool> {
    let auth_headers = keycloak::get_client_credentials().await?;

    // The trustee name is simply the username of the user
    let trustee_name = claims
        .trustee
        .ok_or(anyhow!("trustee name not found"))?;

    // get the keys ceremonies for this election event
    let keys_ceremony = get_keys_ceremony_by_id(
        &auth_headers.clone(),
        &tenant_id.clone(),
        &election_event_id.clone(),
        &keys_ceremony_id,
    )
    .await?;
    // check keys_ceremony has correct execution status
    if keys_ceremony.execution_status != Some(ExecutionStatus::IN_PROCESS.to_string())
        && keys_ceremony.execution_status != Some(ExecutionStatus::SUCCESS.to_string())
    {
        return Err(anyhow!(
            "Keys ceremony not in ExecutionStatus::IN_PROCESS or  ExecutionStatus::SUCCESS"
        ));
    }

    // get ceremony status
    let current_status: CeremonyStatus = deserialize_value(
        keys_ceremony
            .status
            .clone()
            .ok_or(anyhow!("Empty keys ceremony status"))?,
    )
    .with_context(|| "error parsing keys ceremony current status")?;

    // check the trustee is part of this ceremony
    if let None = current_status.trustees.clone().into_iter().find(|trustee| {
        (trustee.name == trustee_name
            && (trustee.status == TrusteeStatus::KEY_GENERATED
                || trustee.status == TrusteeStatus::KEY_RETRIEVED
                || trustee.status == TrusteeStatus::KEY_CHECKED))
    }) {
        return Err(anyhow!(
            "Trustee not part of the keys ceremony or has invalid state"
        ));
    }

    // get the encrypted private key
    let encrypted_private_key =
        find_trustee_private_key(&auth_headers, &tenant_id, &election_event_id, &trustee_name)
            .await?;

    if encrypted_private_key != private_key_base64 {
        return Ok(false);
    }

    // Update ceremony with the information that this trustee did get the
    // private key
    let new_status = CeremonyStatus {
        stop_date: None,
        public_key: current_status.public_key.clone(),
        logs: append_keys_trustee_check_log(&current_status.logs, &trustee_name),
        trustees: current_status
            .trustees
            .iter()
            .map(|trustee| {
                if (trustee.name == trustee_name) {
                    Ok(Trustee {
                        name: trustee.name.clone(),
                        status: TrusteeStatus::KEY_CHECKED,
                    })
                } else {
                    Ok(trustee.clone())
                }
            })
            .collect::<Result<Vec<Trustee>>>()?,
    };

    let all_trustees_checked = new_status
        .trustees
        .iter()
        .all(|trustee| trustee.status == TrusteeStatus::KEY_CHECKED);
    let new_execution_status = if all_trustees_checked {
        ExecutionStatus::SUCCESS
    } else {
        ExecutionStatus::IN_PROCESS
    };

    // update keys-ceremony into the database using graphql
    update_keys_ceremony_status(
        auth_headers.clone(),
        tenant_id.clone(),
        election_event_id.clone(),
        keys_ceremony_id.clone(),
        /* status */ serde_json::to_value(new_status)?,
        /* execution_status */ new_execution_status.to_string(),
    )
    .await
    .with_context(|| "couldn't update keys ceremony")?;

    if ExecutionStatus::SUCCESS == new_execution_status {
        // get the election event
        let election_event = get_election_event_helper(
            auth_headers.clone(),
            tenant_id.clone(),
            election_event_id.clone(),
        )
        .await?;
        let current_status = get_election_event_status(election_event.status).unwrap();
        let mut new_status = current_status.clone();
        new_status.keys_ceremony_finished = Some(true);
        let new_status_js = serde_json::to_value(new_status)?;
        update_election_event_status(
            auth_headers.clone(),
            tenant_id.clone(),
            election_event_id.clone(),
            new_status_js,
        )
        .await?;
    }

    event!(
        Level::INFO,
        "Retrieved private key for electionEventId={}, keysCeremonyId={}, trusteeName={}",
        election_event_id.clone(),
        keys_ceremony_id.clone(),
        trustee_name.clone()
    );
    Ok(true)
}

#[instrument(err)]
pub async fn create_keys_ceremony(
    tenant_id: String,
    election_event_id: String,
    threshold: usize,
    trustee_names: Vec<String>,
) -> Result<String> {
    let auth_headers = keycloak::get_client_credentials().await?;
    let celery_app = get_celery_app().await;
    // verify trustee names and fetch their objects to get their ids
    let trustees = get_trustees_by_name(&auth_headers, &tenant_id, &trustee_names)
        .await?
        .data
        .with_context(|| "can't find trustees")?
        .sequent_backend_trustee;

    if trustee_names.len() != trustees.len() {
        return Err(anyhow!("can't find trustees"));
    }
    if threshold < 2 || threshold > trustees.len() {
        return Err(anyhow!("invalid threshold"));
    }

    // obtain trustee ids list
    let trustee_ids = trustees
        .clone()
        .into_iter()
        .map(|trustee| trustee.id)
        .collect();

    // get the election event
    let election_event = get_election_event_helper(
        auth_headers.clone(),
        tenant_id.clone(),
        election_event_id.clone(),
    )
    .await?;

    // find if there's any previous ceremony and if so, stop. shouldn't happen,
    // we only allow one per election event
    let keys_ceremonies = get_keys_ceremonies(
        auth_headers.clone(),
        tenant_id.clone(),
        election_event_id.clone(),
    )
    .await?
    .data
    .with_context(|| "error listing existing keys ceremonies")?
    .sequent_backend_keys_ceremony;
    let has_any_running_ceremony = keys_ceremonies.len() > 0;
    if has_any_running_ceremony {
        return Err(anyhow!("there's already an existing running ceremony"));
    }

    // generate default values
    let keys_ceremony_id: String = Uuid::new_v4().to_string();
    let execution_status: String = ExecutionStatus::NOT_STARTED.to_string();
    let status: Value = serde_json::to_value(CeremonyStatus {
        stop_date: None,
        public_key: None,
        logs: generate_keys_initial_log(&trustee_names),
        trustees: trustees
            .clone()
            .into_iter()
            .map(|trustee| {
                Ok(Trustee {
                    name: trustee.name.ok_or(anyhow!("empty trustee name"))?,
                    status: TrusteeStatus::WAITING,
                })
            })
            .collect::<Result<Vec<Trustee>>>()?,
    })?;

    // insert keys-ceremony into the database using graphql
    insert_keys_ceremony(
        auth_headers.clone(),
        keys_ceremony_id.clone(),
        tenant_id.clone(),
        election_event_id.clone(),
        trustee_ids,
        /* threshold */ threshold.try_into()?,
        /* status */ Some(status),
        /* execution_status */ Some(execution_status),
    )
    .await
    .with_context(|| "couldn't insert keys ceremony")?;

    // create the public keys in async task
    let task = celery_app
        .send_task(create_keys::new(
            CreateKeysBody {
                threshold: threshold,
                trustee_pks: trustees
                    .clone()
                    .into_iter()
                    .map(|trustee| {
                        Ok(trustee.public_key.ok_or(anyhow!("empty trustee pub key"))?)
                    })
                    .collect::<Result<Vec<String>>>()?,
            },
            tenant_id.clone(),
            election_event_id.clone(),
        ))
        .await?;
    event!(Level::INFO, "Sent create_keys task {}", task.task_id);

    // Save it in the electoral log
    let board_name = get_election_event_board(election_event.bulletin_board_reference.clone())
        .with_context(|| "missing bulletin board")?;

    let electoral_log = ElectoralLog::new(board_name.as_str()).await?;
    electoral_log
        .post_keygen(election_event_id.clone())
        .await
        .with_context(|| "error posting to the electoral log")?;

    Ok(keys_ceremony_id)
}
