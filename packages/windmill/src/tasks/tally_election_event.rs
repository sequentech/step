// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::{anyhow, Context};
use braid_messages::newtypes::BatchNumber;
use celery::prelude::*;
use sequent_core::services::openid;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use tracing::{event, instrument, Level};

use crate::hasura::area::get_election_event_areas;
use crate::hasura::tally_session::{get_tally_session_highest_batch, insert_tally_session};
use crate::hasura::tally_session_contest::insert_tally_session_contest;
use crate::hasura::trustee::get_trustees_by_id;
use crate::services::celery_app::get_celery_app;
use crate::tasks::insert_ballots::{insert_ballots, InsertBallotsPayload};
use crate::types::task_error::into_task_error;
use crate::types::error::{Error, Result};

#[derive(Deserialize, Debug, Serialize, Clone)]
pub struct TallyElectionBody {
    election_ids: Vec<String>,
    trustee_ids: Vec<String>,
}

#[instrument]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task]
pub async fn tally_election_event(
    body: TallyElectionBody,
    tenant_id: String,
    election_event_id: String,
) -> Result<()> {
    let auth_headers = openid::get_client_credentials()
        .await?;

    let areas_data = get_election_event_areas(
        auth_headers.clone(),
        tenant_id.clone(),
        election_event_id.clone(),
        body.election_ids.clone(),
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
    let trustees = get_trustees_by_id(
        auth_headers.clone(),
        tenant_id.clone(),
        body.trustee_ids.clone(),
    )
    .await?
    .data
    .with_context(|| "can't find trustees")?;

    let tally_session = insert_tally_session(
        auth_headers.clone(),
        tenant_id.clone(),
        election_event_id.clone(),
        body.election_ids.clone(),
        body.trustee_ids.clone(),
        area_ids.clone(),
    )
    .await?
    .data
    .with_context(|| "can't find tally session")?
    .insert_sequent_backend_tally_session
    .ok_or(anyhow!("can't find tally session"))?
    .returning[0]
        .clone();

    let mut batch: BatchNumber = get_tally_session_highest_batch(
        auth_headers.clone(),
        tenant_id.clone(),
        election_event_id.clone(),
    )
    .await?;
    let celery_app = get_celery_app().await;

    for area_contest in contest_areas.into_iter() {
        let tally_session_contest = insert_tally_session_contest(
            auth_headers.clone(),
            tenant_id.clone(),
            election_event_id.clone(),
            area_contest.area_id.clone().unwrap(),
            area_contest.contest_id.clone().unwrap(),
            batch.clone(),
            tally_session.id.clone(),
        )
        .await?
        .data
        .with_context(|| "can't insert tally session contest")?
        .insert_sequent_backend_tally_session_contest
        .ok_or(anyhow!("can't find tally session contest"))?
        .returning[0]
            .clone();
        let task = celery_app
            .send_task(insert_ballots::new(
                InsertBallotsPayload {
                    trustee_pks: vec![],
                },
                tenant_id.clone(),
                election_event_id.clone(),
                tally_session.id.clone(),
                tally_session_contest.id.clone(),
            ))
            .await?;
        event!(Level::INFO, "Sent INSERT_BALLOTS task {}", task.task_id);
        batch = batch + 1;
    }
    Ok(())
}
