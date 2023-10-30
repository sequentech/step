// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::Context;
use anyhow::Result;
use celery::prelude::*;
use sequent_core::services::openid;
use tracing::{event, instrument, Level};
use braid_messages::newtypes::BatchNumber;

use crate::hasura::area_contest::get_area_contests;
use crate::services::celery_app::get_celery_app;
use crate::tasks::insert_ballots::{InsertBallotsPayload, insert_ballots};
use crate::types::task_error::into_task_error;

#[instrument]
#[celery::task]
pub async fn tally_election_event_area(
    tenant_id: String,
    election_event_id: String,
    area_id: String,
) -> TaskResult<()> {
    let auth_headers = openid::get_client_credentials()
        .await
        .map_err(into_task_error)?;

    let area_contests =
        get_area_contests(
            auth_headers,
            tenant_id.clone(),
            election_event_id.clone(),
            area_id.clone()
        )
            .await
            .map_err(into_task_error)?
            .data
            .with_context(|| "can't find election event areas")
            .map_err(into_task_error)?;
    let celery_app = get_celery_app().await;

    let mut batch: BatchNumber = 0;

    for area_contest in area_contests.sequent_backend_area_contest.iter() {
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
        event!(Level::INFO, "Sent TALLY_ELECTION_EVENT_AREA task {}", task.task_id);
        batch = batch + 1;
    }
    Ok(())
}