// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::Result;
use tracing::{event, instrument, Level};
use windmill::services::celery_app::get_celery_app;
use windmill::tasks::create_ballot_style;
use windmill::tasks::create_board;
use windmill::tasks::create_keys;
use windmill::tasks::insert_ballots;
use windmill::tasks::render_report;
use windmill::tasks::set_public_key::*;
use windmill::tasks::tally_election_event::{
    tally_election_event, TallyElectionBody,
};
use windmill::tasks::update_election_event_ballot_styles::update_election_event_ballot_styles;
use windmill::tasks::update_voting_status;
use windmill::types::scheduled_event::*;
use uuid::Uuid;

use crate::routes::scheduled_event;
use crate::services::worker::scheduled_event::CreateEventBody;

#[instrument]
pub async fn process_scheduled_event(event: CreateEventBody) -> Result<()> {
    let celery_app = get_celery_app().await;
    match event.event_processor.clone() {
        EventProcessors::CREATE_REPORT => {
            let body: render_report::RenderTemplateBody =
                serde_json::from_value(event.event_payload.clone())?;
            let task = celery_app
                .send_task(render_report::render_report::new(
                    body,
                    event.tenant_id,
                    event.election_event_id,
                    Uuid::new_v4(),
                ))
                .await?;
            event!(Level::INFO, "Sent CREATE_REPORT task {}", task.task_id);
        }
        EventProcessors::UPDATE_VOTING_STATUS => {
            let payload: update_voting_status::UpdateVotingStatusPayload =
                serde_json::from_value(event.event_payload.clone())?;
            let task = celery_app
                .send_task(update_voting_status::update_voting_status::new(
                    payload,
                    event.tenant_id,
                    event.election_event_id,
                ))
                .await?;
            event!(
                Level::INFO,
                "Sent UPDATE_VOTING_STATUS task {}",
                task.task_id
            );
        }
        EventProcessors::CREATE_BOARD => {
            let task = celery_app
                .send_task(create_board::create_board::new(
                    event.tenant_id,
                    event.election_event_id,
                ))
                .await?;
            event!(Level::INFO, "Sent CREATE_BOARD task {}", task.task_id);
        }
        EventProcessors::CREATE_KEYS => {
            let payload: create_keys::CreateKeysBody =
                serde_json::from_value(event.event_payload.clone())?;
            let task = celery_app
                .send_task(create_keys::create_keys::new(
                    payload,
                    event.tenant_id,
                    event.election_event_id,
                ))
                .await?;
            event!(Level::INFO, "Sent CREATE_KEYS task {}", task.task_id);
        }
        EventProcessors::SET_PUBLIC_KEY => {
            let task = celery_app
                .send_task(set_public_key::new(
                    event.tenant_id,
                    event.election_event_id,
                ))
                .await?;
            event!(Level::INFO, "Sent SET_PUBLIC_KEY task {}", task.task_id);
        }
        EventProcessors::TALLY_ELECTION_EVENT => {
            let payload: TallyElectionBody =
                serde_json::from_value(event.event_payload.clone())?;
            let task = celery_app
                .send_task(tally_election_event::new(
                    payload,
                    event.tenant_id,
                    event.election_event_id.clone(),
                ))
                .await?;
            event!(
                Level::INFO,
                "Sent TALLY_ELECTION_EVENT task {}",
                task.task_id
            );
        }
        EventProcessors::CREATE_ELECTION_EVENT_BALLOT_STYLES => {
            let task = celery_app
                .send_task(update_election_event_ballot_styles::new(
                    event.tenant_id,
                    event.election_event_id.clone(),
                ))
                .await?;
            event!(
                Level::INFO,
                "Sent CREATE_ELECTION_EVENT_BALLOT_STYLES task {}",
                task.task_id
            );
        }
    }
    Ok(())
}
