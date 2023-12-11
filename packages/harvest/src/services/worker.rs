// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::routes::scheduled_event;
use crate::services::authorization::authorize;
use crate::services::worker::scheduled_event::CreateEventBody;
use anyhow::{Result, anyhow};
use sequent_core::services::jwt::JwtClaims;
use sequent_core::types::permissions::Permissions;
use tracing::{event, instrument, Level};
use uuid::Uuid;
use windmill::services::celery_app::get_celery_app;
use windmill::tasks::create_keys;
use windmill::tasks::render_report;
use windmill::tasks::send_communication::*;
use windmill::tasks::set_public_key::*;
use windmill::tasks::update_election_event_ballot_styles::update_election_event_ballot_styles;
use windmill::tasks::update_voting_status;
use windmill::types::scheduled_event::*;

#[instrument(skip(claims))]
pub async fn process_scheduled_event(
    event: CreateEventBody,
    claims: JwtClaims,
) -> Result<String> {
    let celery_app = get_celery_app().await;
    let element_id: String = Uuid::new_v4().to_string();
    match event.event_processor.clone() {
        EventProcessors::CREATE_REPORT => {
            let body: render_report::RenderTemplateBody =
                serde_json::from_value(event.event_payload.clone())?;
            let election_event_id = event.election_event_id.ok_or(anyhow!("empty election_event_id"))?;
            let task = celery_app
                .send_task(render_report::render_report::new(
                    body,
                    event.tenant_id,
                    election_event_id,
                ))
                .await?;
            event!(Level::INFO, "Sent CREATE_REPORT task {}", task.task_id);
        }
        EventProcessors::UPDATE_VOTING_STATUS => {
            let payload: update_voting_status::UpdateVotingStatusPayload =
                serde_json::from_value(event.event_payload.clone())?;
            let election_event_id = event.election_event_id.ok_or(anyhow!("empty election_event_id"))?;
                let task = celery_app
                .send_task(update_voting_status::update_voting_status::new(
                    payload,
                    event.tenant_id,
                    election_event_id,
                ))
                .await?;
            event!(
                Level::INFO,
                "Sent UPDATE_VOTING_STATUS task {}",
                task.task_id
            );
        }
        EventProcessors::SET_PUBLIC_KEY => {
            let election_event_id = event.election_event_id.ok_or(anyhow!("empty election_event_id"))?;
            let task = celery_app
                .send_task(set_public_key::new(
                    event.tenant_id,
                    election_event_id,
                ))
                .await?;
            event!(Level::INFO, "Sent SET_PUBLIC_KEY task {}", task.task_id);
        }
        EventProcessors::CREATE_ELECTION_EVENT_BALLOT_STYLES => {
            /*let task = celery_app
                .send_task(update_election_event_ballot_styles::new(
                    event.tenant_id,
                    event.election_event_id.clone(),
                ))
                .await?;
            event!(
                Level::INFO,
                "Sent CREATE_ELECTION_EVENT_BALLOT_STYLES task {}",
                task.task_id
            );*/
        }
        EventProcessors::SEND_COMMUNICATION => {
            let payload: SendCommunicationBody =
                serde_json::from_value(event.event_payload.clone())?;
            let task = celery_app
                .send_task(send_communication::new(
                    payload,
                    event.tenant_id,
                    event.election_event_id.clone(),
                ))
                .await?;
            event!(
                Level::INFO,
                "Sent SEND_COMMUNICATION task {}",
                task.task_id
            );
        }
    }
    Ok(element_id)
}
