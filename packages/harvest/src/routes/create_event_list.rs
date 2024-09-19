// SPDX-FileCopyrightText: 2024 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only


use serde::{Deserialize, Serialize};
use tracing::{event, instrument, Level};

#[derive(Serialize, Deserialize, Debug)]

pub struct GetEventListInput {
    election_event_id: String,
    tenant_id: String,
    election_id: String,
}

#[derive(Serialize, Deserialize, Debug)]

pub struct GetEventListOutput {
    election: String,
    schedule: Jsonb,
    task_id: String,
    tenant_id: String,
    election_event_id: String,
    event_type: String,
    receivers: [String],
    template: Jsonb
}

#[instrument]
#[post("/get_event_list", format = "json", data = "<body>")]
pub async fn list_electoral_log(
    body: Json<ElectionEventStatsInput>,
    claims: JwtClaims,
) -> Result<Json<GetEventListOutput>, (Status, String)> {
    let input = body.into_inner();
    authorize(
        &claims,
        true,
        Some(input.tenant_id.clone()),
        vec![Permissions::LOGS_READ],
    )?;
    let ret_val = get_logs(input)
        .await
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;

    Ok(Json(ret_val))
}