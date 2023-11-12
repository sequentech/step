// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::Context;
use celery::error::TaskError;
use sequent_core::services::keycloak;
use tracing::{event, instrument, Level};

use crate::hasura::area::get_election_event_areas;
use crate::services::celery_app::get_celery_app;
use crate::services::redis::get_lock_manager;
use crate::tasks::create_ballot_style::create_ballot_style;
use crate::tasks::create_ballot_style::CreateBallotStylePayload;
use crate::types::error::Result;

#[instrument]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task]
pub async fn update_election_event_ballot_styles(
    tenant_id: String,
    election_event_id: String,
) -> Result<()> {
    let rl = get_lock_manager();
    // Create the lock
    let lock_opt = rl
        .lock(
            format!(
                "update_election_event_ballot_styles-{}-{}",
                tenant_id, election_event_id
            )
            .as_bytes(),
            1000,
        )
        .await;
    let lock = match lock_opt {
        Ok(lock) => lock,
        Err(_) => {
            event!(Level::INFO, "Ending early as task is locked");
            return Ok(());
        }
    };
    let auth_headers = keycloak::get_client_credentials().await?;

    let areas = get_election_event_areas(
        auth_headers,
        tenant_id.clone(),
        election_event_id.clone(),
        vec![],
    )
    .await?
    .data
    .with_context(|| "can't find election event areas")?;
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
            .await?;
        event!(
            Level::INFO,
            "Sent CREATE_BALLOT_STYLE task {}",
            task.task_id
        );
    }
    // Remove the lock
    rl.unlock(&lock).await;
    Ok(())
}
