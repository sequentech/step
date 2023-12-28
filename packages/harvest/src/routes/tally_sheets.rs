// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::authorization::authorize;
use anyhow::Result;
use rocket::http::Status;
use rocket::serde::json::Json;
use sequent_core::services::jwt::JwtClaims;
use sequent_core::types::ceremonies::TallyExecutionStatus;
use sequent_core::types::permissions::Permissions;
use serde::{Deserialize, Serialize};
use tracing::{event, instrument, Level};
use windmill::services::ceremonies::tally_ceremony;

#[derive(Serialize, Deserialize, Debug)]
pub struct PublishTallySheetInput {
    election_event_id: String,
    tally_sheet_id: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PublishTallySheetOutput {
    tally_sheet_id: String,
}

// The main function to start a key ceremony
#[instrument(skip(claims))]
#[post("/publish-tally-sheet", format = "json", data = "<body>")]
pub async fn publish_tally_sheet(
    body: Json<PublishTallySheetInput>,
    claims: JwtClaims,
) -> Result<Json<PublishTallySheetOutput>, (Status, String)> {
    authorize(&claims, true, None, vec![Permissions::TALLY_SHEET_PUBLISH])?;
    let input = body.into_inner();

    Ok(Json(PublishTallySheetOutput {
        tally_sheet_id: input.tally_sheet_id,
    }))
}
