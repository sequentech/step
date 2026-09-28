// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::authorization::authorize;
use deadpool_postgres::Client as DbClient;
use protocol_board::{
    TrusteeBoardsResponse, TrusteeReport, TrusteeReportResponse,
};
use rocket::http::Status;
use rocket::serde::json::Json;
use sequent_core::services::jwt::JwtClaims;
use sequent_core::types::permissions::Permissions;
use tracing::instrument;
use windmill::services::database::get_hasura_pool;
use windmill::services::trustee_boards;

/// The boards the calling trustee has work on, with their kind and parent.
#[instrument(skip(claims))]
#[get("/trustee/boards")]
pub async fn list_trustee_boards(
    claims: JwtClaims,
) -> Result<Json<TrusteeBoardsResponse>, (Status, String)> {
    authorize(
        &claims,
        true,
        Some(claims.hasura_claims.tenant_id.clone()),
        vec![Permissions::TRUSTEE_CEREMONY],
    )?;
    let tenant_id = claims.hasura_claims.tenant_id.clone();

    let mut hasura_db_client: DbClient = get_hasura_pool()
        .await
        .get()
        .await
        .map_err(|e| (Status::InternalServerError, format!("{e:?}")))?;
    let hasura_transaction = hasura_db_client
        .transaction()
        .await
        .map_err(|e| (Status::InternalServerError, format!("{e:?}")))?;

    let boards = trustee_boards::list_boards(
        &hasura_transaction,
        &tenant_id,
        claims.trustee.as_deref(),
    )
    .await
    .map_err(|e| (Status::InternalServerError, format!("{e:?}")))?;

    Ok(Json(boards))
}

/// The calling trustee's report about one of its boards, answered with the
/// state the board's ceremony is in once the report was applied.
#[instrument(skip(claims))]
#[post("/trustee/boards/report", format = "json", data = "<body>")]
pub async fn report_trustee_board(
    body: Json<TrusteeReport>,
    claims: JwtClaims,
) -> Result<Json<TrusteeReportResponse>, (Status, String)> {
    authorize(
        &claims,
        true,
        Some(claims.hasura_claims.tenant_id.clone()),
        vec![Permissions::TRUSTEE_CEREMONY],
    )?;
    let tenant_id = claims.hasura_claims.tenant_id.clone();

    let mut hasura_db_client: DbClient = get_hasura_pool()
        .await
        .get()
        .await
        .map_err(|e| (Status::InternalServerError, format!("{e:?}")))?;
    let hasura_transaction = hasura_db_client
        .transaction()
        .await
        .map_err(|e| (Status::InternalServerError, format!("{e:?}")))?;

    let answer = trustee_boards::report(
        hasura_transaction,
        &tenant_id,
        claims.trustee.as_deref(),
        body.into_inner(),
    )
    .await
    .map_err(|e| (Status::InternalServerError, format!("{e:?}")))?;

    Ok(Json(answer))
}
