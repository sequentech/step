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
use windmill::tasks::set_public_key::*;
use windmill::types::scheduled_event::*;

use crate::hasura::event_execution;
use crate::routes::scheduled_event;
use crate::services::celery;
use crate::services::events::create_ballot_style;
use crate::services::events::create_board;
use crate::services::events::create_keys;
use crate::services::events::insert_ballots;
use crate::services::events::render_report;
use crate::services::events::update_voting_status;

#[derive(Debug, Clone)]
struct CustomError;

impl fmt::Display for CustomError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Custom error")
    }
}

impl Error for CustomError {}

#[instrument(skip_all)]
async fn insert_event_execution_with_result(
    auth_headers: connection::AuthHeaders,
    event: ScheduledEvent,
    result_payload: Option<Value>,
) -> Result<event_execution::EventExecution> {
    let insert_event_execution = event_execution::insert_event_execution(
        auth_headers,
        event.tenant_id.unwrap(),
        event.election_event_id.unwrap(),
        event.id,
        event_execution::EventExecutionState::Success,
        event.event_payload.unwrap(),
        result_payload,
    )
    .await?;

    let event_execution = &insert_event_execution
        .data
        .expect("expected data".into())
        .insert_sequent_backend_event_execution
        .unwrap()
        .returning[0];
    Ok(event_execution::EventExecution {
        id: event_execution.id.clone(),
        tenant_id: event_execution.tenant_id.clone(),
        election_event_id: event_execution.election_event_id.clone(),
        scheduled_event_id: event_execution.scheduled_event_id.clone(),
        labels: event_execution.labels.clone(),
        annotations: event_execution.annotations.clone(),
        execution_state: event_execution.execution_state.clone().map(|s| {
            event_execution::EventExecutionState::from_str(s.as_str()).unwrap()
        }),
        execution_payload: event_execution.execution_payload.clone(),
        result_payload: event_execution.result_payload.clone(),
        started_at: event_execution.started_at.clone(),
        ended_at: event_execution.ended_at.clone(),
    })
}

#[instrument(skip(auth_headers))]
pub async fn process_scheduled_event(
    auth_headers: connection::AuthHeaders,
    event: ScheduledEvent,
) -> Result<event_execution::EventExecution> {
    let celery_app = celery::create_app().await?;
    match event.clone().event_processor.unwrap() {
        EventProcessors::CREATE_REPORT => {
            let body: render_report::RenderTemplateBody =
                serde_json::from_value(event.event_payload.clone().unwrap())?;
            let document_json =
                render_report::render_report(Json(body), auth_headers.clone())
                    .await?;
            let document = document_json.into_inner();
            let document_value = serde_json::to_value(document)?;

            insert_event_execution_with_result(
                auth_headers,
                event,
                Some(document_value),
            )
            .await
        }
        EventProcessors::UPDATE_VOTING_STATUS => {
            let payload: update_voting_status::UpdateVotingStatusPayload =
                serde_json::from_value(event.event_payload.clone().unwrap())?;
            let _update_result = update_voting_status::update_voting_status(
                auth_headers.clone(),
                event.tenant_id.clone().unwrap(),
                event.election_event_id.clone().unwrap(),
                payload,
            )
            .await?;

            insert_event_execution_with_result(auth_headers, event, None).await
        }
        EventProcessors::CREATE_BOARD => {
            let payload: create_board::CreateBoardPayload =
                serde_json::from_value(event.event_payload.clone().unwrap())?;
            let board = create_board::create_board(
                auth_headers.clone(),
                event.tenant_id.clone().unwrap().as_str(),
                event.election_event_id.clone().unwrap().as_str(),
                payload.board_name.as_str(),
            )
            .await?;
            let board_value = serde_json::to_value(board)?;

            insert_event_execution_with_result(
                auth_headers,
                event,
                Some(board_value),
            )
            .await
        }
        EventProcessors::CREATE_KEYS => {
            let payload: create_keys::CreateKeysBody =
                serde_json::from_value(event.event_payload.clone().unwrap())?;
            create_keys::create_keys(
                auth_headers.clone(),
                payload,
                event.clone(),
                celery_app,
            )
            .await?;

            insert_event_execution_with_result(auth_headers, event, None).await
        }
        EventProcessors::SET_PUBLIC_KEY => {
            let task = celery_app
                .send_task(set_public_key_task::new(
                    auth_headers.clone(),
                    event.clone(),
                ))
                .await?;
            event!(Level::INFO, "Sent SET_PUBLIC_KEY task {}", task.task_id);

            insert_event_execution_with_result(auth_headers, event, None).await
        }
        EventProcessors::CREATE_BALLOT_STYLE => {
            let payload: create_ballot_style::CreateBallotStylePayload =
                serde_json::from_value(event.event_payload.clone().unwrap())?;
            create_ballot_style::create_ballot_style(
                auth_headers.clone(),
                payload,
                event.clone(),
            )
            .await?;

            insert_event_execution_with_result(auth_headers, event, None).await
        }
        EventProcessors::INSERT_BALLOTS => {
            let payload: insert_ballots::InsertBallotsPayload =
                serde_json::from_value(event.event_payload.clone().unwrap())?;
            insert_ballots::insert_ballots(
                auth_headers.clone(),
                payload,
                event.clone(),
            )
            .await?;

            insert_event_execution_with_result(auth_headers, event, None).await
        }
    }
}
