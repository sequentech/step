// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::authorization::authorize_voter_election;
use crate::types::error_response::{ErrorCode, ErrorResponse, JsonError};
use anyhow::Result;
use electoral_log::client::types::OrderDirection;
use electoral_log::client::types::*;
use rocket::http::Status;
use rocket::serde::json::Json;
use sequent_core::ballot::ShowCastVoteLogs;
use sequent_core::services::jwt::JwtClaims;
use sequent_core::types::hasura::core::ElectionEvent;
use sequent_core::types::permissions::VoterPermissions;
use serde::Deserialize;
use std::collections::HashMap;
use tracing::instrument;
use windmill::postgres::election_event::get_election_event_by_id;
use windmill::services::electoral_log;
use windmill::services::electoral_log::{
    CastVoteMessagesOutput, GetElectoralLogBody, OrderField,
};
use windmill::services::providers::transactions_provider::provide_hasura_transaction;

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
#[post("/immudb/list-cast-vote-messages", format = "json", data = "<body>")]
pub async fn list_cast_vote_messages(
    body: Json<CastVoteMessagesInput>,
    claims: JwtClaims,
) -> Result<Json<CastVoteMessagesOutput>, JsonError> {
    let input = body.into_inner();
    // let election_id = input.election_id.as_deref().unwrap_or_default();
    let election_id = input.election_id.clone().unwrap_or_default(); // TODO: Temporary till merging the ballot performace inprovements.
    let username = claims.preferred_username.clone().unwrap_or_default();
    let user_id = claims.hasura_claims.user_id.clone();

    // Check auth.
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

    // Check that the policy is enabled
    provide_hasura_transaction(|hasura_transaction| {
        let tenant_id = claims.hasura_claims.tenant_id.clone();
        let election_event_id = input.election_event_id.clone();
        Box::pin(async move {
            let election_event: ElectionEvent = get_election_event_by_id(
                hasura_transaction,
                &tenant_id,
                &election_event_id,
            )
            .await?;
            let policy = election_event.presentation.and_then(
                |val| val.get("show_cast_vote_logs")
                    .map(|value| serde_json::from_value::<ShowCastVoteLogs>(value.clone()).unwrap_or_default())
            ).unwrap_or_default();
            match policy {
                ShowCastVoteLogs::ShowLogsTab => {
                    Ok(())
                }
                ShowCastVoteLogs::HideLogsTab => {
                    Err(anyhow::anyhow!(ShowCastVoteLogs::HideLogsTab.to_string()))
                }
            }
        })
    })
    .await
    .map_err(|error| {
        ErrorResponse::new(
            Status::Forbidden,
            &format!("Failed to confirm that the show_cast_vote_logs policy is enabled: {error:?}"),
            ErrorCode::ConfirmPolicyShowCastVoteLogsFailed,
        )
    })?;

    let ballot_id = input.ballot_id.as_str();
    let elog_input = GetElectoralLogBody {
        tenant_id: input.tenant_id,
        election_event_id: input.election_event_id,
        limit: input.limit,
        offset: input.offset,
        order_by: input.order_by,
        election_id: input.election_id,
        ..Default::default()
    };

    let ret_val = electoral_log::list_cast_vote_messages(
        elog_input, ballot_id, &user_id, &username,
    )
    .await
    .map_err(|e| {
        ErrorResponse::new(
            Status::InternalServerError,
            &format!("Error to list cast vote messages: {e:?}"),
            ErrorCode::InternalServerError,
        )
    })?;

    Ok(Json(ret_val))
}
