// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::authorization::authorize;
use anyhow::{Context, Result};
use deadpool_postgres::Client as DbClient;
use rocket::http::Status;
use rocket::serde::json::Json;
use sequent_core::services::jwt::JwtClaims;
use sequent_core::types::permissions::Permissions;
use serde::{Deserialize, Serialize};
use tracing::instrument;
use windmill::postgres::tally_sheet;
use windmill::services::database::get_hasura_pool;

#[derive(Serialize, Deserialize, Debug)]
pub struct PublishTallySheetInput {
    election_event_id: String,
    tally_sheet_id: String,
    publish: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PublishTallySheetOutput {
    tally_sheet_id: Option<String>,
}

// The main function to start a key ceremony
#[instrument(skip(claims))]
#[post("/publish-tally-sheet", format = "json", data = "<body>")]
pub async fn publish_tally_sheet(
    body: Json<PublishTallySheetInput>,
    claims: JwtClaims,
) -> Result<Json<PublishTallySheetOutput>, (Status, String)> {
    authorize(
        &claims,
        true,
        Some(claims.hasura_claims.tenant_id.clone()),
        vec![Permissions::TALLY_SHEET_PUBLISH],
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

    let found = tally_sheet::publish_tally_sheet(
        &hasura_transaction,
        &claims.hasura_claims.tenant_id,
        &input.election_event_id,
        &input.tally_sheet_id,
        &claims.hasura_claims.user_id,
        input.publish,
    )
    .await
    .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;

    if found.is_none() {
        return Ok(Json(PublishTallySheetOutput {
            tally_sheet_id: None,
        }));
    }

    hasura_transaction
        .commit()
        .await
        .with_context(|| "error comitting transaction")
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;

    Ok(Json(PublishTallySheetOutput {
        tally_sheet_id: Some(input.tally_sheet_id.clone()),
    }))
}
