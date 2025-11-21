// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::authorization::authorize;
use anyhow::{Context, Result};
use deadpool_postgres::Client as DbClient;
use rocket::http::Status;
use rocket::serde::json::Json;
use sequent_core::services::jwt::JwtClaims;
use sequent_core::types::hasura::core::TallySheet;
use sequent_core::types::permissions::Permissions;
use sequent_core::types::tally_sheets::{
    AreaContestResults, TallySheetStatus, VotingChannel,
};
use serde::{Deserialize, Serialize};
use tracing::instrument;
use windmill::postgres::{contest::get_contest_by_id, tally_sheet};
use windmill::services::database::get_hasura_pool;

#[derive(Serialize, Deserialize, Debug)]
pub struct CreateNewTallySheetInput {
    election_event_id: String,
    channel: VotingChannel,
    content: AreaContestResults,
    contest_id: String,
    area_id: String,
}

// The main function to start a key ceremony
#[instrument(skip(claims))]
#[post("/create-new-tally-sheet", format = "json", data = "<body>")]
pub async fn create_new_tally_sheet(
    body: Json<CreateNewTallySheetInput>,
    claims: JwtClaims,
) -> Result<Json<TallySheet>, (Status, String)> {
    authorize(
        &claims,
        true,
        Some(claims.hasura_claims.tenant_id.clone()),
        vec![Permissions::TALLY_SHEET_CREATE],
    )?;
    let input = body.into_inner();

    let mut hasura_db_client: DbClient = get_hasura_pool()
        .await
        .get()
        .await
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;

    let hasura_transaction = hasura_db_client
        .transaction()
        .await
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;

    let contest_opt = get_contest_by_id(
        &hasura_transaction,
        &claims.hasura_claims.tenant_id,
        &input.election_event_id,
        &input.contest_id,
    )
    .await
    .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;

    let Some(contest) = contest_opt else {
        return Err((
            Status::NotFound,
            format!("Contest {} not found ", input.contest_id),
        ));
    };

    let version = tally_sheet::get_latest_ballot_box_version(
        &hasura_transaction,
        &claims.hasura_claims.tenant_id,
        &input.election_event_id,
        &contest.election_id,
        &input.area_id,
        &input.contest_id,
        &input.channel,
    )
    .await
    .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;

    let new_tally_sheet = tally_sheet::insert_tally_sheet(
        &hasura_transaction,
        &claims.hasura_claims.tenant_id,
        &input.election_event_id,
        &contest.election_id,
        &input.contest_id,
        &input.area_id,
        &input.content,
        &input.channel,
        &claims.hasura_claims.user_id,
        TallySheetStatus::PENDING,
        version + 1,
    )
    .await
    .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;

    hasura_transaction
        .commit()
        .await
        .with_context(|| "error comitting transaction")
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;

    Ok(Json(new_tally_sheet.clone()))
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ReviewTallySheetInput {
    election_event_id: String,
    tally_sheet_id: String,
    new_status: TallySheetStatus,
}

// The main function to start a key ceremony
#[instrument(skip(claims))]
#[post("/review-tally-sheet", format = "json", data = "<body>")]
pub async fn review_tally_sheet(
    body: Json<ReviewTallySheetInput>,
    claims: JwtClaims,
) -> Result<Json<TallySheet>, (Status, String)> {
    authorize(
        &claims,
        true,
        Some(claims.hasura_claims.tenant_id.clone()),
        vec![Permissions::TALLY_SHEET_REVIEW],
    )?;
    let input = body.into_inner();
    let mut hasura_db_client: DbClient = get_hasura_pool()
        .await
        .get()
        .await
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;

    let hasura_transaction = hasura_db_client
        .transaction()
        .await
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;

    let tally_sheet_opt = tally_sheet::review_tally_sheet_status(
        &hasura_transaction,
        &claims.hasura_claims.tenant_id,
        &input.election_event_id,
        &input.tally_sheet_id,
        &claims.hasura_claims.user_id,
        input.new_status.clone(),
    )
    .await
    .map_err(|e| (Status::InternalServerError, format!("{e:?}")))?;

    let tally_sheet = match tally_sheet_opt {
        Some(t) => t,
        None => {
            return Err((
                Status::NotFound,
                "Tally sheet not found".to_string(),
            ));
        }
    };

    if input.new_status == TallySheetStatus::APPROVED {
        tally_sheet::soft_delete_tally_sheet_leftover_versions(
            &hasura_transaction,
            &tally_sheet,
        )
        .await
        .map_err(|e| (Status::InternalServerError, format!("{e:?}")))?;
    }

    hasura_transaction
        .commit()
        .await
        .with_context(|| "error comitting transaction")
        .map_err(|e| (Status::InternalServerError, format!("{e:?}")))?;

    Ok(Json(tally_sheet.clone()))
}
