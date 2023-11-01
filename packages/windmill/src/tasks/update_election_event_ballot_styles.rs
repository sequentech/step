// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::Context;
use celery::prelude::*;
use sequent_core::services::openid;
use tracing::{event, instrument, Level};

use crate::hasura::area::get_election_event_areas;
use crate::services::celery_app::get_celery_app;
use crate::tasks::create_ballot_style::create_ballot_style;
use crate::tasks::create_ballot_style::CreateBallotStylePayload;
use crate::types::task_error::into_task_error;

#[instrument]
#[celery::task]
pub async fn update_election_event_ballot_styles(
    tenant_id: String,
    election_event_id: String,
) -> TaskResult<()> {
    let auth_headers = openid::get_client_credentials()
        .await
        .map_err(into_task_error)?;

    let areas =
        get_election_event_areas(auth_headers, tenant_id.clone(), election_event_id.clone())
            .await
            .map_err(into_task_error)?
            .data
            .with_context(|| "can't find election event areas")
            .map_err(into_task_error)?;
    let celery_app = get_celery_app().await;

    for area in areas.sequent_backend_area.iter() {
        let task = celery_app
            .send_task(create_ballot_style::new(
                CreateBallotStylePayload {
                    area_id: area.id.clone(),
                },
                tenant_id.clone(),
                election_event_id.clone(),
            ))
            .await
            .map_err(into_task_error)?;
        event!(
            Level::INFO,
            "Sent CREATE_BALLOT_STYLE task {}",
            task.task_id
        );
    }
    Ok(())
}
