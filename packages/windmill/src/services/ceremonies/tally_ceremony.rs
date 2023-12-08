// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::hasura::area::get_election_event_areas;
use crate::hasura::keys_ceremony::get_keys_ceremony;
use crate::hasura::tally_session::{
    get_tally_session_by_id, get_tally_sessions, insert_tally_session, update_tally_session_status,
};
use crate::hasura::tally_session_execution::{
    get_last_tally_session_execution, insert_tally_session_execution,
};
use crate::services::celery_app::get_celery_app;
use crate::services::ceremonies::keys_ceremony::get_keys_ceremony_status;
use crate::services::ceremonies::tally_ceremony::get_keys_ceremony::GetKeysCeremonySequentBackendKeysCeremony;
use crate::services::ceremonies::tally_ceremony::get_last_tally_session_execution::{
    GetLastTallySessionExecutionSequentBackendTallySession,
    GetLastTallySessionExecutionSequentBackendTallySessionExecution,
};
use crate::services::ceremonies::tally_ceremony::get_tally_session_by_id::GetTallySessionByIdSequentBackendTallySession;
use crate::services::ceremonies::tally_ceremony::get_tally_sessions::GetTallySessionsSequentBackendTallySession;
use crate::tasks::connect_tally_ceremony::connect_tally_ceremony;
use anyhow::{anyhow, Context, Result};
use sequent_core::services::connection;
use sequent_core::services::keycloak;
use sequent_core::types::ceremonies::*;
use serde_json::{from_value, Value};
use std::collections::HashSet;
use std::str::FromStr;
use tracing::{event, instrument, Level};
use uuid::Uuid;

pub async fn find_last_tally_session_execution(
    auth_headers: connection::AuthHeaders,
    tenant_id: String,
    election_event_id: String,
    tally_session_id: String,
) -> Result<
    Option<(
        GetLastTallySessionExecutionSequentBackendTallySessionExecution,
        GetLastTallySessionExecutionSequentBackendTallySession,
    )>,
> {
    // get all data for the execution: the last tally session execution,
    // the list of tally_session_contest, and the ballot styles
    let data = get_last_tally_session_execution(
        auth_headers.clone(),
        tenant_id.clone(),
        election_event_id.clone(),
        tally_session_id.clone(),
    )
    .await?
    .data
    .expect("expected data");

    if data.sequent_backend_tally_session.len() == 0 {
        event!(Level::INFO, "Missing tally session");
        return Ok(None);
    }

    if data.sequent_backend_tally_session_execution.len() == 0 {
        event!(Level::INFO, "Missing tally session execution");
        return Ok(None);
    }
    Ok(Some((
        data.sequent_backend_tally_session_execution[0].clone(),
        data.sequent_backend_tally_session[0].clone(),
    )))
}

pub async fn get_tally_session(
    auth_headers: connection::AuthHeaders,
    tenant_id: String,
    election_event_id: String,
    tally_session_id: String,
) -> Result<GetTallySessionByIdSequentBackendTallySession> {
    // fetch tally_sessions
    let tally_session = &get_tally_session_by_id(
        auth_headers.clone(),
        tenant_id.clone(),
        election_event_id.clone(),
        tally_session_id.clone(),
    )
    .await?
    .data
    .expect("expected data")
    .sequent_backend_tally_session[0];

    Ok(tally_session.clone())
}

pub fn get_tally_ceremony_status(input: Option<Value>) -> Result<TallyCeremonyStatus> {
    input
        .map(|value| {
            from_value(value)
                .map_err(|err| anyhow!("Error parsing tally ceremony status: {:?}", err))
        })
        .ok_or(anyhow!("Missing tally ceremony status"))
        .flatten()
}

pub async fn find_keys_ceremony(
    auth_headers: connection::AuthHeaders,
    tenant_id: String,
    election_event_id: String,
) -> Result<GetKeysCeremonySequentBackendKeysCeremony> {
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
    Ok(successful_ceremonies[0].clone())
}

fn generate_initial_tally_status(
    election_ids: &Vec<String>,
    keys_ceremony_status: &CeremonyStatus,
) -> TallyCeremonyStatus {
    TallyCeremonyStatus {
        stop_date: None,
        logs: vec![],
        trustees: keys_ceremony_status
            .trustees
            .iter()
            .map(|trustee| TallyTrustee {
                name: trustee.name.clone(),
                status: TallyTrusteeStatus::WAITING,
            })
            .collect(),
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
    let keys_ceremony = find_keys_ceremony(
        auth_headers.clone(),
        tenant_id.clone(),
        election_event_id.clone(),
    )
    .await?;
    let keys_ceremony_status = get_keys_ceremony_status(keys_ceremony.status)?;
    let keys_ceremony_id = keys_ceremony.id.clone();
    let area_ids = get_area_ids(
        auth_headers.clone(),
        tenant_id.clone(),
        election_event_id.clone(),
        election_ids.clone(),
    )
    .await?;
    let initial_status = generate_initial_tally_status(&election_ids, &keys_ceremony_status);
    let tally_session_id: String = Uuid::new_v4().to_string();
    let _tally_session = insert_tally_session(
        auth_headers.clone(),
        tenant_id.clone(),
        election_event_id.clone(),
        election_ids.clone(),
        area_ids.clone(),
        tally_session_id.clone(),
        keys_ceremony_id.clone(),
        TallyExecutionStatus::NOT_STARTED,
    )
    .await?
    .data
    .with_context(|| "can't find tally session")?
    .insert_sequent_backend_tally_session
    .ok_or(anyhow!("can't find tally session"))?
    .returning[0]
        .clone();

    let _tally_session_execution = insert_tally_session_execution(
        auth_headers.clone(),
        tenant_id.clone(),
        election_event_id.clone(),
        -1,
        tally_session_id.clone(),
        None,
        Some(initial_status),
        None,
    )
    .await?;
    Ok(keys_ceremony_id)
}

pub async fn update_tally_ceremony(
    tenant_id: String,
    election_event_id: String,
    tally_session_id: String,
    new_execution_status: TallyExecutionStatus,
) -> Result<()> {
    let auth_headers = keycloak::get_client_credentials().await?;
    let celery_app = get_celery_app().await;

    let tally_session = get_tally_session(
        auth_headers.clone(),
        tenant_id.clone(),
        election_event_id.clone(),
        tally_session_id.clone(),
    )
    .await?;

    let current_status = tally_session
        .execution_status
        .map(|value| {
            TallyExecutionStatus::from_str(&value).unwrap_or(TallyExecutionStatus::NOT_STARTED)
        })
        .unwrap_or(TallyExecutionStatus::NOT_STARTED);

    let expected_status = match current_status {
        TallyExecutionStatus::NOT_STARTED => vec![
            TallyExecutionStatus::STARTED,
            TallyExecutionStatus::CANCELLED,
        ],
        TallyExecutionStatus::STARTED => vec![
            TallyExecutionStatus::CONNECTED,
            TallyExecutionStatus::CANCELLED,
        ],
        TallyExecutionStatus::CONNECTED => vec![
            TallyExecutionStatus::IN_PROGRESS,
            TallyExecutionStatus::CANCELLED,
        ],
        TallyExecutionStatus::IN_PROGRESS => vec![
            TallyExecutionStatus::SUCCESS,
            TallyExecutionStatus::CANCELLED,
        ],
        TallyExecutionStatus::SUCCESS => vec![
            TallyExecutionStatus::CANCELLED,
            TallyExecutionStatus::CANCELLED,
        ],
        TallyExecutionStatus::CANCELLED => vec![TallyExecutionStatus::CANCELLED],
    };

    if !expected_status.contains(&new_execution_status) {
        return Err(anyhow!("Unexpected status"));
    }

    update_tally_session_status(
        auth_headers.clone(),
        tenant_id.clone(),
        election_event_id.clone(),
        tally_session_id.clone(),
        new_execution_status.clone(),
    )
    .await?;

    if TallyExecutionStatus::STARTED == new_execution_status {
        // "connect" trustees in async task
        let task = celery_app
            .send_task(connect_tally_ceremony::new(
                tenant_id.clone(),
                election_event_id.clone(),
                tally_session_id.clone(),
            ))
            .await?;
        event!(
            Level::INFO,
            "Sent connect_tally_ceremony task {}",
            task.task_id
        );
    }

    Ok(())
}
