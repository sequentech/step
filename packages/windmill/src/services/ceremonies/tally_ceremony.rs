// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::hasura::area::get_election_event_areas;
use crate::hasura::keys_ceremony::get_keys_ceremony;
use crate::hasura::tally_session::insert_tally_session;
use crate::services::celery_app::get_celery_app;
use crate::tasks::connect_tally_ceremony::connect_tally_ceremony;
use anyhow::{anyhow, Context, Result};
use sequent_core::services::connection;
use sequent_core::services::keycloak;
use sequent_core::types::ceremonies::*;
use std::collections::HashSet;
use uuid::Uuid;
use tracing::{event, instrument, Level};

pub async fn find_keys_ceremony(
    auth_headers: connection::AuthHeaders,
    tenant_id: String,
    election_event_id: String,
) -> Result<String> {
    // find if there's any previous ceremony. There should be one and it should
    // have finished successfully.
    let keys_ceremonies = get_keys_ceremony(
        auth_headers.clone(),
        tenant_id.clone(),
        election_event_id.clone(),
    )
    .await?
    .data
    .with_context(|| "error listing existing keys ceremonies")?
    .sequent_backend_keys_ceremony;

    let successful_ceremonies: Vec<_> = keys_ceremonies
        .into_iter()
        .filter(|ceremony| {
            ceremony
                .execution_status
                .clone()
                .map(|value| value == ExecutionStatus::SUCCESS.to_string())
                .unwrap_or(false)
        })
        .collect();
    if 0 == successful_ceremonies.len() {
        return Err(anyhow!("Can't find keys ceremony"));
    }
    if successful_ceremonies.len() > 1 {
        return Err(anyhow!("Expected a single keys ceremony"));
    }
    Ok(successful_ceremonies[0].id.clone())
}

fn generate_initial_tally_status(election_ids: &Vec<String>) -> TallyCeremonyStatus {
    TallyCeremonyStatus {
        stop_date: None,
        logs: vec![],
        trustees: vec![],
        elections_status: election_ids
            .iter()
            .map(|election_id| TallyElection {
                election_id: election_id.clone(),
                status: TallyElectionStatus::WAITING,
                progress: 0.0,
            })
            .collect(),
    }
}

// get area ids that are linked to these election ids
pub async fn get_area_ids(
    auth_headers: connection::AuthHeaders,
    tenant_id: String,
    election_event_id: String,
    election_ids: Vec<String>,
) -> Result<Vec<String>> {
    let areas_data = get_election_event_areas(
        auth_headers.clone(),
        tenant_id.clone(),
        election_event_id.clone(),
        election_ids.clone(),
    )
    .await?
    .data
    .with_context(|| "can't find election event areas")?;
    let contest_ids = areas_data
        .sequent_backend_contest
        .into_iter()
        .map(|contest| contest.id)
        .collect::<Vec<_>>();
    let contest_areas = areas_data
        .sequent_backend_area_contest
        .into_iter()
        .filter(|contest_area| {
            contest_area
                .contest_id
                .clone()
                .map(|contest_id| contest_ids.contains(&contest_id))
                .unwrap_or(false)
        })
        .collect::<Vec<_>>();
    let area_ids = contest_areas
        .clone()
        .into_iter()
        .filter(|contest_area| contest_area.area_id.is_some())
        .map(|contest_area| contest_area.area_id.unwrap())
        .collect::<HashSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();

    Ok(area_ids)
}

pub async fn create_tally_ceremony(
    tenant_id: String,
    election_event_id: String,
    election_ids: Vec<String>,
) -> Result<String> {
    let auth_headers = keycloak::get_client_credentials().await?;
    let celery_app = get_celery_app().await;
    let keys_ceremony_id = find_keys_ceremony(
        auth_headers.clone(),
        tenant_id.clone(),
        election_event_id.clone(),
    )
    .await?;
    let area_ids = get_area_ids(
        auth_headers.clone(),
        tenant_id.clone(),
        election_event_id.clone(),
        election_ids.clone(),
    )
    .await?;
    let initial_status = generate_initial_tally_status(&election_ids);
    let tally_session_id: String = Uuid::new_v4().to_string();
    let _tally_session = insert_tally_session(
        auth_headers.clone(),
        tenant_id.clone(),
        election_event_id.clone(),
        election_ids.clone(),
        vec![], // trustee_ids
        area_ids.clone(),
        tally_session_id.clone(),
        keys_ceremony_id.clone(),
        TallyExecutionStatus::NOT_STARTED,
        initial_status,
    )
    .await?
    .data
    .with_context(|| "can't find tally session")?
    .insert_sequent_backend_tally_session
    .ok_or(anyhow!("can't find tally session"))?
    .returning[0]
        .clone();

    // create the public keys in async task
    let task = celery_app
        .send_task(connect_tally_ceremony::new(
            tenant_id.clone(),
            election_event_id.clone(),
            tally_session_id.clone(),
        ))
        .await?;
    event!(Level::INFO, "Sent connect_tally_ceremony task {}", task.task_id);
    Ok(keys_ceremony_id)
}
