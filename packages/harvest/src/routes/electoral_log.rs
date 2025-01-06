// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::authorization::authorize;
use anyhow::{anyhow, Context, Result};
use deadpool_postgres::Client as DbClient;
use rocket::http::Status;
use rocket::serde::json::Json;
use sequent_core::services::jwt::JwtClaims;
use sequent_core::types::permissions::Permissions;
use serde::{Deserialize, Serialize};
use tracing::instrument;
use windmill::postgres::election_event::get_election_event_by_id;
use windmill::services::database::get_hasura_pool;
use windmill::services::election_event_board::get_election_event_board;
use windmill::services::electoral_log::{
    list_electoral_log as get_logs, ElectoralLog, ElectoralLogRow,
    GetElectoralLogBody,
};
use windmill::types::resources::DataList;

const EVENT_TYPE_COMMUNICATIONS: &str = "communications";

#[derive(Serialize, Deserialize, Debug)]
pub struct LogEventInput {
    election_event_id: String,
    message_type: String,
    user_id: Option<String>,
    body: String,
}
#[derive(Serialize, Deserialize, Debug)]
pub struct LogEventOutput {
    id: String,
}

#[instrument]
#[post("/immudb/electoral-log", format = "json", data = "<body>")]
pub async fn list_electoral_log(
    body: Json<GetElectoralLogBody>,
    claims: JwtClaims,
) -> Result<Json<DataList<ElectoralLogRow>>, (Status, String)> {
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

#[instrument]
#[post("/immudb/log-event", format = "json", data = "<body>")]
pub async fn create_electoral_log(
    body: Json<LogEventInput>,
    claims: JwtClaims,
) -> Result<Json<LogEventOutput>, (Status, String)> {
    let input = body.into_inner();
    authorize(
        &claims,
        true,
        Some(claims.hasura_claims.tenant_id.clone()),
        vec![Permissions::SERVICE_ACCOUNT],
    )?;
    let mut hasura_db_client: DbClient = get_hasura_pool()
        .await
        .get()
        .await
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;

    let hasura_transaction = hasura_db_client
        .transaction()
        .await
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;

    let election_event = get_election_event_by_id(
        &hasura_transaction,
        &claims.hasura_claims.tenant_id,
        &input.election_event_id,
    )
    .await
    .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;
    info!("log event election event: {:?}", election_event);
    let board_name = get_election_event_board(
        election_event.bulletin_board_reference.clone(),
    )
    .with_context(|| "error getting election event")
    .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;
    let tenant_id = claims.hasura_claims.tenant_id;
    let user_id = claims.hasura_claims.user_id;
    let username = claims.preferred_username;
    let electoral_log =
        ElectoralLog::for_admin_user(&board_name, &tenant_id, &user_id)
            .await
            .map_err(|e| {
                (
                    Status::InternalServerError,
                    format!("error getting electoral log: {e:?}"),
                )
            })?;

    if input.body.contains(EVENT_TYPE_COMMUNICATIONS) {
        let body = input
            .body
            .replace(EVENT_TYPE_COMMUNICATIONS, "")
            .trim()
            .to_string();
        let _ = electoral_log
            .post_send_template(
                Some(body),
                input.election_event_id.clone(),
                Some(user_id.clone()),
                username.clone(),
                None,
            )
            .await
            .map_err(|e| anyhow!("error posting to the electoral log {e:?}"));
    } else {
        electoral_log
            .post_keycloak_event(
                input.election_event_id.clone(),
                input.message_type,
                input.body,
                Some(user_id.clone()),
                username.clone(),
            )
            .await
            .map_err(|e| {
                (
                    Status::InternalServerError,
                    format!("error posting registration error: {e:?}"),
                )
            })?;
    }
    Ok(Json(LogEventOutput {
        id: input.election_event_id.clone(),
    }))
}
