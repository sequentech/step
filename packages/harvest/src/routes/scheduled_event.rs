// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::Result;
use rocket::response::Debug;
use rocket::serde::json::Json;
use sequent_core::services::connection;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::instrument;
use windmill::types::scheduled_event::*;

use crate::services;

#[derive(Deserialize, Debug, Clone)]
pub struct CreateEventBody {
    pub tenant_id: String,
    pub election_event_id: String,
    pub event_processor: EventProcessors,
    pub cron_config: Option<String>,
    pub event_payload: Value,
    pub created_by: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CreateEventOutput {
    pub id: String,
}

#[instrument(skip(_auth_headers))]
#[post("/scheduled-event", format = "json", data = "<body>")]
pub async fn create_scheduled_event(
    body: Json<CreateEventBody>,
    _auth_headers: connection::AuthHeaders,
) -> Result<Json<CreateEventOutput>, Debug<anyhow::Error>> {
    let input = body.into_inner();

    services::worker::process_scheduled_event(input.clone()).await?;

    Ok(Json(CreateEventOutput {
        id: input.tenant_id,
    }))
}
