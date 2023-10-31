// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::Context;
use anyhow::Result;
use braid_messages::newtypes::BatchNumber;
use celery::prelude::*;
use sequent_core::services::openid;
use tracing::{event, instrument, Level};

use crate::hasura::area::get_election_event_areas;
use crate::hasura::trustee::get_trustees_by_id;
use crate::hasura::tally_session::{get_tally_session_highest_batch, insert_tally_session};
use crate::services::celery_app::get_celery_app;
use crate::tasks::tally_election_event_area::tally_election_event_area;
use crate::types::task_error::into_task_error;

#[instrument]
#[celery::task]
pub async fn tally_election_event(
    tenant_id: String,
    election_event_id: String,
    election_ids: Vec<String>,
    trustee_ids: Vec<String>,
) -> TaskResult<()> {
    let auth_headers = openid::get_client_credentials()
        .await
        .map_err(into_task_error)?;

    let areas_data = get_election_event_areas(
        auth_headers,
        tenant_id.clone(),
        election_ids.clone(),
        election_event_id.clone(),
    )
    .await
    .map_err(into_task_error)?
    .data
    .with_context(|| "can't find election event areas")
    .map_err(into_task_error)?;

    let contest_ids = areas_data.sequent_backend_contest.map(|contest| contest.id);
    let contest_areas = areas_data
        .sequent_backend_area_contest
        .into_iter()
        .filter(|contest_area| contest_ids.contains(contest_area.contest_id))
        .collect();
    let area_ids = contest_areas
        .into_iter()
        .map(|contest_area| contest_area.area_id)
        .collect::<HashSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let trustees = get_trustees_by_id(auth_headers.clone(), tenant_id.clone(), trustee_ids.clone())
        .await
        .map_err(into_task_error)?
        .data
        .with_context(|| "can't find trustees")
        .map_err(into_task_error)?;
    
    insert_tally_session(
        auth_headers.clone(),
        tenant_id.clone(),
        election_event_id.clone(),
        election_ids.clone(),
        trustee_ids.clone(),
        area_ids.clone()
    ).await.map_err(into_task_error)?;

    let mut batch: BatchNumber =
        get_tally_session_highest_batch(auth_headers, tenant_id.clone(), election_event_id.clone())
            .await?;
    let celery_app = get_celery_app().await;

    for area_contest in contest_areas.iter() {
        let task = celery_app
            .send_task(insert_ballots::new(
                InsertBallotsPayload {
                    trustee_pks: vec![],
                },
                tenant_id.clone(),
                election_event_id.clone(),
                area_id.clone(),
                area_contest.contest_id.clone().unwrap(),
                batch.clone(),
            ))
            .await
            .map_err(into_task_error)?;
        event!(Level::INFO, "Sent INSERT_BALLOTS task {}", task.task_id);
        batch = batch + 1;
    }
    Ok(())
}
