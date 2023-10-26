// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::Result;
use rocket::response::Debug;
use rocket::serde::json::Json;
use serde_json::Value;
use serde::{Deserialize, Serialize};
use sequent_core::services::connection;
use std::str::FromStr;
use strum_macros::Display;
use strum_macros::EnumString;
use tracing::instrument;
use windmill::hasura;
use windmill::types::scheduled_event::*;

use crate::services;

#[derive(Deserialize, Debug)]
pub struct CreateScheduledEventBody {
    tenant_id: String,
    election_event_id: String,
    event_processor: EventProcessors,
    cron_config: Option<String>,
    event_payload: Value,
    created_by: String,
}

#[instrument(skip(auth_headers))]
#[post("/scheduled-event", format = "json", data = "<body>")]
pub async fn create_scheduled_event(
    body: Json<CreateScheduledEventBody>,
    auth_headers: connection::AuthHeaders,
) -> Result<Json<ScheduledEvent>, Debug<anyhow::Error>> {
    let input = body.into_inner();
    let scheduled_event_result =
        hasura::scheduled_event::insert_scheduled_event(
            auth_headers.clone(),
            input.tenant_id.clone(),
            input.election_event_id.clone(),
            input.event_processor.clone().to_string(),
            input.cron_config.clone(),
            input.event_payload.clone(),
            input.created_by.clone(),
        )
        .await?;

    let scheduled_event = &scheduled_event_result
        .data
        .unwrap()
        .insert_sequent_backend_scheduled_event
        .unwrap()
        .returning[0];

    let formatted_event = ScheduledEvent {
        id: scheduled_event.id.clone(),
        tenant_id: scheduled_event.tenant_id.clone(),
        election_event_id: scheduled_event.election_event_id.clone(),
        created_at: scheduled_event.created_at.clone(),
        stopped_at: scheduled_event.stopped_at.clone(),
        labels: scheduled_event.labels.clone(),
        annotations: scheduled_event.annotations.clone(),
        event_processor: scheduled_event
            .event_processor
            .clone()
            .map(|p| EventProcessors::from_str(p.as_str()).unwrap()),
        cron_config: scheduled_event.cron_config.clone(),
        event_payload: scheduled_event.event_payload.clone(),
        created_by: scheduled_event.created_by.clone(),
    };

    println!(
        "formatted_event payload: {}",
        formatted_event.event_payload.clone().unwrap()
    );

    services::worker::process_scheduled_event(
        auth_headers,
        formatted_event.clone(),
    )
    .await?;

    Ok(Json(formatted_event))
}
