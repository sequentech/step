// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::authorization::authorize_voter_election;
use crate::types::error_response::{ErrorCode, ErrorResponse, JsonError};
use anyhow::{anyhow, Context, Result};
use deadpool_postgres::Client as DbClient;
use rocket::http::Status;
use rocket::serde::json::Json;
use sequent_core::services::jwt::JwtClaims;
use sequent_core::types::permissions::Permissions;
use sequent_core::types::permissions::VoterPermissions;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::instrument;
use windmill::services::electoral_log::{
    list_cast_vote_messages, CastVoteMessagesOutput, GetElectoralLogBody,
    OrderField,
};
use windmill::types::resources::OrderDirection;

#[derive(Deserialize, Debug)]
pub struct CastVoteMessagesInput {
    pub tenant_id: String,
    pub election_event_id: String,
    pub election_id: Option<String>,
    pub ballot_id: String,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub order_by: Option<HashMap<OrderField, OrderDirection>>,
}

#[instrument]
#[post("/immudb/cast-vote-messages", format = "json", data = "<body>")]
pub async fn cast_vote_messages(
    body: Json<CastVoteMessagesInput>,
    claims: JwtClaims,
) -> Result<Json<CastVoteMessagesOutput>, JsonError> {
    let input = body.into_inner();
    // let election_id = input.election_id.as_deref().unwrap_or_default();
    let election_id = input.election_id.clone().unwrap_or_default(); // TODO: Temporary till merging the ballot performace inprovements.
    let (_area_id, _voting_channel) = authorize_voter_election(
        &claims,
        vec![VoterPermissions::CAST_VOTE],
        &election_id,
    )
    .map_err(|e| {
        ErrorResponse::new(
            Status::Unauthorized,
            &format!("{:?}", e),
            ErrorCode::Unauthorized,
        )
    })?; // TODO: Temporary till merging the ballot performace inprovements.

    let ballot_id = input.ballot_id.as_str();
    let mut filter_map: HashMap<OrderField, String> = HashMap::new();
    filter_map.insert(OrderField::StatementKind, "CastVote".to_string());
    let elog_input = GetElectoralLogBody {
        tenant_id: input.tenant_id,
        election_event_id: input.election_event_id,
        limit: input.limit,
        offset: input.offset,
        filter: Some(filter_map),
        order_by: input.order_by,
        election_id: input.election_id,
        area_ids: None,
        only_with_user: None, //???
    };

    let ret_val = list_cast_vote_messages(elog_input, ballot_id)
        .await
        .map_err(|e| {
            ErrorResponse::new(
                Status::InternalServerError,
                &format!("{:?}", e),
                ErrorCode::InternalServerError,
            )
        })?;

    Ok(Json(ret_val))
}
