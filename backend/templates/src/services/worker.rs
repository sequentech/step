// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use reqwest;
use rocket::serde::json::Json;
use std::str::FromStr;
use strum_macros::EnumString;
use anyhow::Result;
use std::fmt;
use std::error::Error;

use crate::connection;
use crate::hasura::event_execution;
use crate::services::events::render_report;
use crate::services::events::create_board;
use crate::routes::scheduled_event;
use crate::services::events::update_voting_status;

#[derive(Debug, Clone)]
struct CustomError;

impl fmt::Display for CustomError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
      write!(f, "Custom error")
    }
}

impl Error for CustomError { }

pub async fn process_scheduled_event(
    auth_headers: connection::AuthHeaders,
    event: scheduled_event::ScheduledEvent,
) -> Result<event_execution::EventExecution> {
    match event.event_processor.unwrap() {
        scheduled_event::EventProcessors::CREATE_REPORT => {
            let body: render_report::RenderTemplateBody =
                serde_json::from_value(event.event_payload.clone().unwrap())?;
            let document_json =
                render_report::render_report(Json(body), auth_headers.clone())
                    .await?;
            let document = document_json.into_inner();
            let document_value = serde_json::to_value(document)?;
            let insert_event_execution =
                event_execution::insert_event_execution(
                    auth_headers,
                    event.tenant_id.unwrap(),
                    event.election_event_id.unwrap(),
                    event.id,
                    event_execution::EventExecutionState::Success,
                    event.event_payload.unwrap(),
                    Some(document_value),
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
                    event_execution::EventExecutionState::from_str(s.as_str())
                        .unwrap()
                }),
                execution_payload: event_execution.execution_payload.clone(),
                result_payload: event_execution.result_payload.clone(),
                started_at: event_execution.started_at.clone(),
                ended_at: event_execution.ended_at.clone(),
            })
        },
        scheduled_event::EventProcessors::UPDATE_VOTING_STATUS => {
            let payload: update_voting_status::UpdateVotingStatusPayload =
                serde_json::from_value(event.event_payload.clone().unwrap())?;
            let _update_result = update_voting_status::update_voting_status(
                auth_headers.clone(),
                event.tenant_id.clone().unwrap(),
                event.election_event_id.clone().unwrap(),
                payload
            ).await?;
            let insert_event_execution =
                event_execution::insert_event_execution(
                    auth_headers,
                    event.tenant_id.unwrap(),
                    event.election_event_id.unwrap(),
                    event.id,
                    event_execution::EventExecutionState::Success,
                    event.event_payload.unwrap(),
                    None,
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
                    event_execution::EventExecutionState::from_str(s.as_str())
                        .unwrap()
                }),
                execution_payload: event_execution.execution_payload.clone(),
                result_payload: event_execution.result_payload.clone(),
                started_at: event_execution.started_at.clone(),
                ended_at: event_execution.ended_at.clone(),
            })
        },
        scheduled_event::EventProcessors::CREATE_BOARD => {
            let payload: create_board::CreateBoardPayload =
                serde_json::from_value(event.event_payload.clone().unwrap())?;
            let board = create_board::create_board(payload.board_name.as_str()).await?;
            let board_value = serde_json::to_value(board)?;
            let insert_event_execution =
                event_execution::insert_event_execution(
                    auth_headers,
                    event.tenant_id.unwrap(),
                    event.election_event_id.unwrap(),
                    event.id,
                    event_execution::EventExecutionState::Success,
                    event.event_payload.unwrap(),
                    Some(board_value),
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
                    event_execution::EventExecutionState::from_str(s.as_str())
                        .unwrap()
                }),
                execution_payload: event_execution.execution_payload.clone(),
                result_payload: event_execution.result_payload.clone(),
                started_at: event_execution.started_at.clone(),
                ended_at: event_execution.ended_at.clone(),
            })
        }
    }
}
