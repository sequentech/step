// SPDX-FileCopyrightText: 2024 Kevin Nguyen <kevin@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::authorization::authorize_voter;
use anyhow::Result;
use rocket::http::Status;
use rocket::serde::json::Json;
use sequent_core::types::date_time::DateFormat;
use sequent_core::types::permissions::VoterPermissions;
use sequent_core::{services::jwt::JwtClaims, types::date_time::TimeZone};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::time::Instant;
use tracing::{event, instrument, Level};
use uuid::Uuid;
use windmill::services::celery_app::get_celery_app;

#[derive(Serialize, Deserialize, Debug)]
pub struct CreateVoteReceiptInput {
    ballot_id: String,
    ballot_tracker_url: String,
    tenant_id: String,
    election_event_id: String,
    election_id: String,
    time_zone: Option<TimeZone>,
    date_format: Option<DateFormat>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CreateVoteReceiptOutput {
    id: String,
    ballot_id: String,
    status: String,
}

#[instrument(skip_all)]
#[post("/create-vote-receipt", format = "json", data = "<body>")]
pub async fn create_vote_receipt(
    body: Json<CreateVoteReceiptInput>,
    claims: JwtClaims,
) -> Result<Json<CreateVoteReceiptOutput>, (Status, String)> {
    let area_id = authorize_voter(&claims, vec![VoterPermissions::CAST_VOTE])?;
    let voter_id = claims.hasura_claims.user_id.clone();
    let input = body.into_inner();
    let element_id: String = Uuid::new_v4().to_string();
    let celery_app = get_celery_app().await;

    let task = celery_app
        .send_task(
            windmill::tasks::create_vote_receipt::create_vote_receipt::new(
                element_id.clone(),
                input.ballot_id.clone(),
                input.ballot_tracker_url,
                input.tenant_id,
                input.election_event_id,
                input.election_id,
                area_id,
                voter_id,
                input.time_zone,
                input.date_format,
            ),
        )
        .await
        .map_err(|e| {
            (
                Status::InternalServerError,
                format!("Error creating vote receipt: {:?}", e),
            )
        })?;
    event!(Level::INFO, "Sent task {:?} successfully", task);

    Ok(Json(CreateVoteReceiptOutput {
        id: element_id,
        ballot_id: input.ballot_id,
        status: "pending".to_string(),
    }))
}
