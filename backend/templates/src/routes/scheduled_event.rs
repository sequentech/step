// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use rocket::response::Debug;
use rocket::serde::json::Json;
use rocket::serde::json::Value;
use rocket::serde::{Deserialize, Serialize};
use serde_json::json;

use crate::connection;
use crate::s3;
use crate::hasura;

#[derive(Deserialize, Debug, PartialEq, Eq)]
#[serde(crate = "rocket::serde")]
pub enum EventProcessors {
    CREATE_REPORT,
}

#[derive(Deserialize, Debug)]
#[serde(crate = "rocket::serde")]
pub struct CreateScheduledEventBody {
    tenant_id: String,
    election_event_id: String,
    event_processor: EventProcessors
    cron_config: Option<String>
    event_payload: Value
    created_by: String
}


#[derive(Serialize, Deserialize, Debug)]
#[serde(crate = "rocket::serde")]
pub struct CreateScheduledEventResponse {
    id: String,
    tenant_id: Option<String>
    election_event_id: Option<String>
    created_at: Option<Date>
    stopped_at: Option<Date>
    labels: Value
    annotations: Value
    event_processor: EventProcessors
    cron_config: Option<String>
    event_payload: Value
    created_by: Option<String>
}

#[post("/scheduled-event", format = "json", data = "<body>")]
pub async fn create_scheduled_event(
    body: Json<CreateScheduledEventBody>,
    auth_headers: connection::AuthHeaders,
) -> Result<Json<GetDocumentUrlResponse>, Debug<reqwest::Error>> {
    let input = body.into_inner();
    let scheduled_event_result = hasura::scheduled_event::insert_scheduled_event(
        auth_headers,
        input.tenant_id.clone(),
        input.election_event_id.clone(),
        input.document_id.clone(),
    );

    let scheduled_event = &scheduled_event_result
        .data
        .expect("expected data".into())
        .sequent_backend_scheduled_event
        .unwrap()
        .returning[0];

    Ok(Json(CreateScheduledEventResponse {
        id: scheduled_event.id,
        tenant_id: scheduled_event.tenant_id,
        election_event_id: scheduled_event.election_event_id,
        created_at: scheduled_event.created_at,
        stopped_at: scheduled_event.stopped_at,
        labels: scheduled_event.labels,
        annotations: scheduled_event.annotations,
        event_processor: scheduled_event.event_processor,
        cron_config: scheduled_event.cron_config,
        event_payload: scheduled_event.event_payload,
        created_by: scheduled_event.created_by,
    }))
}
