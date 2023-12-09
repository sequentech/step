// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::hasura::election_event::get_election_event;
use crate::hasura::keys_ceremony::{
    get_keys_ceremonies,
    insert_keys_ceremony,
    update_keys_ceremony_status
};
use crate::hasura::trustee::get_trustees_by_name;
use crate::services::celery_app::get_celery_app;
use crate::services::election_event_board::get_election_event_board;
use crate::services::private_keys::get_trustee_encrypted_private_key;
use crate::tasks::create_keys::{create_keys, CreateKeysBody};

use anyhow::{anyhow, Context, Result};
use sequent_core::services::jwt::JwtClaims;
use sequent_core::services::keycloak;
use sequent_core::types::ceremonies::{CeremonyStatus, ExecutionStatus, Trustee, TrusteeStatus};
use serde_json::from_value;
use serde_json::Value;
use tracing::instrument;
use tracing::{event, Level};
use uuid::Uuid;

#[instrument]
pub fn get_keys_ceremony_status(input: Option<Value>) -> Result<CeremonyStatus> {
    input
        .map(|value| {
            from_value(value)
                .map_err(|err| anyhow!("Error parsing keys ceremony status: {:?}", err))
        })
        .ok_or(anyhow!("Missing keys ceremony status"))
        .flatten()
}

#[instrument]
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
        .preferred_username
        .ok_or(anyhow!("username not found"))?;

    // get the keys ceremonies for this election event
    let keys_ceremony =
        get_keys_ceremonies(
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
    if keys_ceremony.execution_status != Some(ExecutionStatus::IN_PROCESS.to_string())
    {
        return Err(
            anyhow!("Keys ceremony not in ExecutionStatus::IN_PROCESS")
        );
    }

    // get ceremony status
    let current_status: CeremonyStatus =
        serde_json::from_value(
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
        return Err(anyhow!(
            "Trustee not part of the keys ceremony"
        ));
    }

    // fetch election_event
    let election_event = 
        &get_election_event(
            auth_headers.clone(),
            tenant_id.clone(),
            election_event_id.clone(),
        )
        .await
        .with_context(|| "error fetching election event")?
        .data
        .with_context(|| "error fetching election event")?
        .sequent_backend_election_event[0];

    // get board name
    let board_name = 
        get_election_event_board(
            election_event.bulletin_board_reference.clone(),
        )
        .with_context(|| "missing bulletin board")?;

    let trustee_public_key = 
        get_trustees_by_name(
            auth_headers.clone(),
            tenant_id.clone(),
            vec![trustee_name.clone()],
        )
        .await
        .with_context(|| "can't find trustee in the database")?
        .data
        .with_context(|| "error fetching election event")?
        .sequent_backend_trustee[0]
        .public_key
        .clone()
        .ok_or(anyhow!(
            "can't get election event"
        ))?;

    // get the encrypted private key
    let encrypted_private_key = 
        get_trustee_encrypted_private_key(
            board_name.as_str(),
            trustee_public_key.as_str(),
        )
        .await?;

    // Update ceremony with the information that this trustee did get the
    // private key
    let status: Value =
        serde_json::to_value(CeremonyStatus {
            stop_date: None,
            public_key: current_status.public_key.clone(),
            logs: current_status.logs.clone(),
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
        /* execution_status */ keys_ceremony
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

#[instrument]
pub async fn create_keys_ceremony(
    tenant_id: String,
    election_event_id: String,
    threshold: usize,
    trustee_names: Vec<String>,
) -> Result<String> {
    let auth_headers = keycloak::get_client_credentials().await?;
    let celery_app = get_celery_app().await;
    // verify trustee names and fetch their objects to get their ids
    let trustees = get_trustees_by_name(
        auth_headers.clone(),
        tenant_id.clone(),
        trustee_names.clone(),
    )
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
    let _election_event = &get_election_event(
        auth_headers.clone(),
        tenant_id.clone(),
        election_event_id.clone(),
    )
    .await
    .with_context(|| "can't get election event")?
    .data
    .ok_or(anyhow!("can't get election event"))?
    .sequent_backend_election_event[0];

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
        logs: vec![],
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
    Ok(keys_ceremony_id)
}
