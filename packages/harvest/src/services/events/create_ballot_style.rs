// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::{Result, Context};
use immu_board::BoardClient;
use rocket::serde::{Deserialize, Serialize};
use sequent_core;
use std::env;
use tracing::instrument;

use crate::connection;
use crate::hasura;
use crate::routes::scheduled_event::ScheduledEvent;

#[derive(Serialize, Deserialize, Debug)]
#[serde(crate = "rocket::serde")]
pub struct CreateBallotStylePayload {
    pub area_id: String,
}


#[instrument(skip(auth_headers))]
pub async fn create_ballot_style(
    auth_headers: connection::AuthHeaders,
    body: CreateBallotStylePayload,
    event: ScheduledEvent,
) -> Result<()> {
    // read tenant_id and election_event_id
    let tenant_id = event
        .tenant_id
        .clone()
        .with_context(|| "scheduled event is missing tenant_id")?;
    let election_event_id = event
        .election_event_id
        .clone()
        .with_context(|| "scheduled event is missing election_event_id")?;
    let hasura_response = hasura::ballot_style::get_ballot_style_area(
        auth_headers,
        tenant_id,
        election_event_id,
        body.area_id
    ).await?;
    Ok(())
}