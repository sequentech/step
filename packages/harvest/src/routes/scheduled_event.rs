// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::Result;
use rocket::response::Debug;
use rocket::serde::json::Json;
use sequent_core::services::connection;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::str::FromStr;
use strum_macros::Display;
use strum_macros::EnumString;
use tracing::instrument;
use windmill::hasura;
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

#[instrument(skip(auth_headers))]
#[post("/scheduled-event", format = "json", data = "<body>")]
pub async fn create_scheduled_event(
    body: Json<CreateEventBody>,
    auth_headers: connection::AuthHeaders,
) -> Result<(), Debug<anyhow::Error>> {
    let input = body.into_inner();

    services::worker::process_scheduled_event(input).await?;

    Ok(())
}
