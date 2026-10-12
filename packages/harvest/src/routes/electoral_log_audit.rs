// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
use crate::services::authorization::authorize;
use rocket::{http::Status, serde::json::Json};
use sequent_core::{
    services::{jwt::JwtClaims, uuid_validation::parse_uuid_v4},
    types::{hasura::core::TasksExecution, permissions::Permissions},
};
use serde::{Deserialize, Serialize};
use tracing::instrument;
use windmill::postgres::election_event::get_election_event_by_id_if_exist;
use windmill::services::database::get_hasura_pool;
use windmill::services::election_event_board::get_election_event_board;
use windmill::services::electoral_log_audit::start_electoral_log_audit;

#[derive(Debug, Deserialize)]
pub struct AuditElectoralLogInput {
    election_event_id: String,
}

#[derive(Serialize)]
pub struct AuditElectoralLogOutput {
    task_execution: TasksExecution,
}

fn internal_error(error: impl std::fmt::Debug) -> (Status, String) {
    tracing::error!("Electoral-log audit request failed: {error:?}");
    (
        Status::InternalServerError,
        "Electoral-log audit request failed".into(),
    )
}

/// Start an audit of an election event's electoral log. The result is recorded on
/// the returned task execution.
#[instrument(skip(claims))]
#[post("/electoral-log/audit", format = "json", data = "<body>")]
pub async fn audit_electoral_log(
    body: Json<AuditElectoralLogInput>,
    claims: JwtClaims,
) -> Result<Json<AuditElectoralLogOutput>, (Status, String)> {
    let tenant_id = claims.hasura_claims.tenant_id.clone();
    authorize(
        &claims,
        true,
        Some(tenant_id.clone()),
        vec![Permissions::ELECTORAL_LOG_AUDIT],
    )?;
    let election_event_id = body.into_inner().election_event_id;
    if parse_uuid_v4(&election_event_id).is_err() {
        return Err((Status::BadRequest, "Invalid election event ID".into()));
    }

    let mut client = get_hasura_pool()
        .await
        .get()
        .await
        .map_err(internal_error)?;
    let transaction = client.transaction().await.map_err(internal_error)?;
    let event = get_election_event_by_id_if_exist(
        &transaction,
        &tenant_id,
        &election_event_id,
    )
    .await
    .map_err(internal_error)?
    .ok_or_else(|| (Status::NotFound, "Election event not found".into()))?;
    transaction.commit().await.map_err(internal_error)?;
    if get_election_event_board(event.bulletin_board_reference).is_none() {
        return Err((
            Status::BadRequest,
            "Election event has no electoral log".into(),
        ));
    }

    let executed_by = claims
        .name
        .clone()
        .unwrap_or_else(|| claims.hasura_claims.user_id.clone());
    let task_execution = start_electoral_log_audit(
        &tenant_id,
        &election_event_id,
        None,
        &executed_by,
    )
    .await
    .map_err(internal_error)?;
    Ok(Json(AuditElectoralLogOutput { task_execution }))
}
