// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::routes::scheduled_event;
use crate::services::worker::scheduled_event::CreateEventBody;
use anyhow::{anyhow, Result};
use sequent_core::serialization::deserialize_with_path::*;
use sequent_core::services::jwt::JwtClaims;
use sequent_core::types::scheduled_event::*;
use sequent_core::types::templates::SendTemplateBody;
use tracing::{event, instrument, Level};
use uuid::Uuid;
use windmill::services::celery_app::get_celery_app;
use windmill::tasks::render_report;
use windmill::tasks::send_template::*;

#[instrument(skip(claims), err)]
pub async fn process_scheduled_event(
    event: CreateEventBody,
    claims: JwtClaims,
) -> Result<String> {
    let celery_app = get_celery_app().await;
    let element_id: String = Uuid::new_v4().to_string();
    match event.event_processor.clone() {
        EventProcessors::CREATE_REPORT => {
            let body: render_report::RenderTemplateBody =
                deserialize_value(event.event_payload.clone())?;
            let election_event_id = event
                .election_event_id
                .ok_or(anyhow!("empty election_event_id"))?;
            let task = celery_app
                .send_task(render_report::render_report::new(
                    body,
                    event.tenant_id,
                    election_event_id,
                ))
                .await?;
            event!(Level::INFO, "Sent CREATE_REPORT task {}", task.task_id);
        }
        EventProcessors::SEND_TEMPLATE => {
            let payload: SendTemplateBody =
                deserialize_value(event.event_payload.clone())?;
            let user_id = claims.hasura_claims.user_id;
            let task = celery_app
                .send_task(send_template::new(
                    payload,
                    event.tenant_id,
                    user_id,
                    event.election_event_id.clone(),
                ))
                .await?;
            event!(Level::INFO, "Sent SEND_TEMPLATE task {}", task.task_id);
        }
        EventProcessors::ALLOW_INIT_REPORT => {}
        EventProcessors::START_VOTING_PERIOD => {}
        EventProcessors::END_VOTING_PERIOD => {}
        EventProcessors::ALLOW_VOTING_PERIOD_END => {}
        EventProcessors::START_ENROLLMENT_PERIOD => {}
        EventProcessors::END_ENROLLMENT_PERIOD => {}
        EventProcessors::START_LOCKDOWN_PERIOD => {}
        EventProcessors::END_LOCKDOWN_PERIOD => {}
        EventProcessors::ALLOW_TALLY => {}
    }
    Ok(element_id)
}
