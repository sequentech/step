// SPDX-FileCopyrightText: 2024 Kevin Nguyen <kevin@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::authorization::authorize_voter;
use anyhow::Result;
use rocket::http::Status;
use rocket::serde::json::Json;
use sequent_core::services::jwt::JwtClaims;
use sequent_core::types::permissions::VoterPermissions;
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
    let start = Instant::now();
    let _area_id = authorize_voter(&claims, vec![VoterPermissions::CAST_VOTE])?;
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
            ),
        )
        .await
        .map_err(|e| {
            let duration = start.elapsed();
            event!(
                Level::INFO,
                "create-vote-receipt took {} ms to complete but failed",
                duration.as_millis()
            );
            (
                Status::InternalServerError,
                format!("Error creating vote receipt: {:?}", e),
            )
        })?;

    event!(
        Level::INFO,
        "Sent CREATE_VOTE_RECEIPT task {}",
        task.task_id
    );

    let duration = start.elapsed();

    event!(
        Level::INFO,
        "create-vote-receipt took {} ms to complete",
        duration.as_millis()
    );

    Ok(Json(CreateVoteReceiptOutput {
        id: element_id,
        ballot_id: input.ballot_id,
        status: "pending".to_string(),
    }))
}
