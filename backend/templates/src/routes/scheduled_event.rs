// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use rocket::response::Debug;
use rocket::serde::json::Json;
use rocket::serde::json::Value;
use rocket::serde::{Deserialize, Serialize};
use serde_json::json;
use std::str::FromStr;
use strum_macros::Display;
use strum_macros::EnumString;

use crate::connection;
use crate::hasura;
use crate::s3;
use crate::services;

#[derive(
    Display, Serialize, Deserialize, Debug, PartialEq, Eq, Clone, EnumString,
)]
#[serde(crate = "rocket::serde")]
pub enum EventProcessors {
    CreateReport,
}

#[derive(Deserialize, Debug)]
#[serde(crate = "rocket::serde")]
pub struct CreateScheduledEventBody {
    tenant_id: String,
    election_event_id: String,
    event_processor: EventProcessors,
    cron_config: Option<String>,
    event_payload: Value,
    created_by: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(crate = "rocket::serde")]
pub struct ScheduledEvent {
    id: String,
    tenant_id: Option<String>,
    election_event_id: Option<String>,
    created_at: Option<String>,
    stopped_at: Option<String>,
    labels: Option<Value>,
    annotations: Option<Value>,
    event_processor: Option<EventProcessors>,
    cron_config: Option<String>,
    event_payload: Option<Value>,
    created_by: Option<String>,
}

#[post("/scheduled-event", format = "json", data = "<body>")]
pub async fn create_scheduled_event(
    body: Json<CreateScheduledEventBody>,
    auth_headers: connection::AuthHeaders,
) -> Result<Json<ScheduledEvent>, Debug<reqwest::Error>> {
    let input = body.into_inner();
    let scheduled_event_result =
        hasura::scheduled_event::insert_scheduled_event(
            auth_headers,
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
        .expect("expected data".into())
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

    services::worker::process_scheduled_event(formatted_event.clone()).await;

    Ok(Json(formatted_event))
}
