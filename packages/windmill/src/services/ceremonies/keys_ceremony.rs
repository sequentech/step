// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::hasura::election_event::get_election_event;
use crate::hasura::keys_ceremony::{get_keys_ceremony, insert_keys_ceremony};
use crate::hasura::trustee::get_trustees_by_name;
use crate::services::celery_app::get_celery_app;
use crate::tasks::create_keys::{create_keys, CreateKeysBody};
use anyhow::{anyhow, Context, Result};
use sequent_core::services::keycloak;
use sequent_core::types::ceremonies::{CeremonyStatus, ExecutionStatus, Trustee, TrusteeStatus};
use serde_json::from_value;
use serde_json::Value;
use tracing::{event, Level};
use uuid::Uuid;

pub fn get_keys_ceremony_status(input: Option<Value>) -> Result<CeremonyStatus> {
    input
        .map(|value| {
            from_value(value)
                .map_err(|err| anyhow!("Error parsing keys ceremony status: {:?}", err))
        })
        .ok_or(anyhow!("Missing keys ceremony status"))
        .flatten()
}

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
    let keys_ceremonies = get_keys_ceremony(
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
