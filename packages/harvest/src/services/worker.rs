// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::Result;
use rocket::serde::json::Json;
use rocket::serde::json::Value;
use std::error::Error;
use std::fmt;
use std::str::FromStr;
use tracing::{event, instrument, Level};
use windmill::connection;
use windmill::services::celery_app::get_celery_app;
use windmill::tasks::create_ballot_style;
use windmill::tasks::create_board;
use windmill::tasks::create_keys;
use windmill::tasks::insert_ballots;
use windmill::tasks::render_report;
use windmill::tasks::set_public_key::*;
use windmill::tasks::update_voting_status;
use windmill::types::scheduled_event::*;
use windmill::hasura::event_execution;

use crate::routes::scheduled_event;

#[instrument(skip(auth_headers))]
pub async fn process_scheduled_event(
    auth_headers: connection::AuthHeaders,
    event: ScheduledEvent,
) -> Result<()> {
    let celery_app = get_celery_app().await;
    match event.clone().event_processor.unwrap() {
        EventProcessors::CREATE_REPORT => {
            let body: render_report::RenderTemplateBody =
                serde_json::from_value(event.event_payload.clone().unwrap())?;
            let task = celery_app
                .send_task(render_report::render_report::new(
                    body,
                    auth_headers.clone(),
                    event.clone(),
                ))
                .await?;
            event!(
                Level::INFO,
                "Sent UPDATE_VOTING_STATUS task {}",
                task.task_id
            );
        }
        EventProcessors::UPDATE_VOTING_STATUS => {
            let payload: update_voting_status::UpdateVotingStatusPayload =
                serde_json::from_value(event.event_payload.clone().unwrap())?;
            let task = celery_app
                .send_task(update_voting_status::update_voting_status::new(
                    auth_headers.clone(),
                    event.clone(),
                    payload,
                ))
                .await?;
            event!(
                Level::INFO,
                "Sent UPDATE_VOTING_STATUS task {}",
                task.task_id
            );
        }
        EventProcessors::CREATE_BOARD => {
            let payload: create_board::CreateBoardPayload =
                serde_json::from_value(event.event_payload.clone().unwrap())?;
            let task = celery_app
                .send_task(create_board::create_board::new(
                    auth_headers.clone(),
                    event.clone(),
                    payload,
                ))
                .await?;
            event!(Level::INFO, "Sent SET_PUBLIC_KEY task {}", task.task_id);
        }
        EventProcessors::CREATE_KEYS => {
            let payload: create_keys::CreateKeysBody =
                serde_json::from_value(event.event_payload.clone().unwrap())?;
            let task = celery_app
                .send_task(create_keys::create_keys::new(
                    auth_headers.clone(),
                    event.clone(),
                    payload,
                ))
                .await?;
            event!(Level::INFO, "Sent CREATE_KEYS task {}", task.task_id);
        }
        EventProcessors::SET_PUBLIC_KEY => {
            let task = celery_app
                .send_task(set_public_key::new(
                    auth_headers.clone(),
                    event.clone(),
                ))
                .await?;
            event!(Level::INFO, "Sent SET_PUBLIC_KEY task {}", task.task_id);
        }
        EventProcessors::CREATE_BALLOT_STYLE => {
            let payload: create_ballot_style::CreateBallotStylePayload =
                serde_json::from_value(event.event_payload.clone().unwrap())?;
            let task = celery_app
                .send_task(create_ballot_style::create_ballot_style::new(
                    auth_headers.clone(),
                    event.clone(),
                    payload,
                ))
                .await?;
            event!(
                Level::INFO,
                "Sent CREATE_BALLOT_STYLE task {}",
                task.task_id
            );
        }
        EventProcessors::INSERT_BALLOTS => {
            let payload: insert_ballots::InsertBallotsPayload =
                serde_json::from_value(event.event_payload.clone().unwrap())?;
            let task = celery_app
                .send_task(insert_ballots::insert_ballots::new(
                    auth_headers.clone(),
                    event.clone(),
                    payload,
                ))
                .await?;
            event!(Level::INFO, "Sent INSERT_BALLOTS task {}", task.task_id);
        }
    }
    Ok(())
}
