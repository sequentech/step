// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::services::authorization::authorize;
use crate::services::dependencies::HarvestServices;
use crate::services::signing_gate::{
    caller, signing_required, waiting, Guarded,
};
use anyhow::Result;
use deadpool_postgres::Client as DbClient;
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::State;
use sequent_core::ballot::{VotingStatus, VotingStatusChannel};
use sequent_core::services::jwt::{has_gold_permission, JwtClaims};
use sequent_core::types::permissions::Permissions;
use sequent_core::types::tally_sheets::VotingChannel;
use serde::{Deserialize, Serialize};
use tracing::instrument;
use windmill::services::signing::action_title;
use windmill::services::signing::actions::voting::{
    action_of, gate_election_status,
};
use windmill::services::signing::actions::{event_ids, is_required};
use windmill::services::signing::guard::SigningRequestSummary;
use windmill::services::{election_event_status, voting_status};
use windmill::tasks::signing_log_outbox::kick_signing_log_outbox;

#[derive(Serialize, Deserialize, Debug)]
pub struct UpdateEventVotingStatusInput {
    pub election_event_id: String,
    pub voting_status: VotingStatus,
    pub voting_channels: Option<Vec<VotingStatusChannel>>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct UpdateEventVotingStatusOutput {
    pub election_event_id: String,
}

#[instrument(skip(claims, services))]
#[post("/update-event-voting-status", format = "json", data = "<body>")]
pub async fn update_event_status(
    body: Json<UpdateEventVotingStatusInput>,
    claims: JwtClaims,
    services: &State<HarvestServices>,
) -> Result<Json<UpdateEventVotingStatusOutput>, Guarded<(Status, String)>> {
    // Check if the user has the required "Gold" role
    if !has_gold_permission(&claims) {
        return Err(
            (Status::Forbidden, "Insufficient privileges".to_string()).into()
        );
    }

    authorize(
        &claims,
        true,
        Some(claims.hasura_claims.tenant_id.clone()),
        vec![Permissions::ELECTION_STATE_WRITE],
    )?;

    let input = body.into_inner();
    let tenant_id = &claims.hasura_claims.tenant_id;
    let user_id = claims.hasura_claims.user_id;
    let username = claims.preferred_username;

    let mut hasura_db_client: DbClient =
        services.databases.hasura().await.get().await.map_err(|e| {
            (
                Status::InternalServerError,
                format!("Error getting hasura client {:?}", e),
            )
        })?;
    let hasura_transaction = hasura_db_client
        .transaction()
        .await
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;

    // While opening or closing a Post needs signatures, the whole event
    // doesn't open or close at once. Pausing is not a signing action.
    if let (Some(action), Some((tenant, event))) = (
        action_of(&input.voting_status),
        event_ids(tenant_id, &input.election_event_id),
    ) {
        if is_required(&hasura_transaction, tenant, event, action)
            .await
            .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?
        {
            return Err(Guarded::Signing(signing_required(action_title(
                action,
            ))));
        }
    }

    election_event_status::update_event_voting_status(
        &hasura_transaction,
        tenant_id,
        Some(&user_id),
        username.as_deref(),
        &input.election_event_id,
        &input.voting_status,
        &input.voting_channels,
    )
    .await
    .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;

    let _commit = hasura_transaction
        .commit()
        .await
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;

    Ok(Json(UpdateEventVotingStatusOutput {
        election_event_id: input.election_event_id.clone(),
    }))
}

/// What `/update-election-voting-status` answers: the Post, and the signing
/// request when opening or closing it needs signatures.
#[derive(Serialize, Debug)]
pub struct UpdateElectionVotingStatusOutput {
    pub election_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signing_request: Option<SigningRequestSummary>,
}

#[instrument(skip(claims, services))]
#[post("/update-election-voting-status", format = "json", data = "<body>")]
pub async fn update_election_status(
    body: Json<voting_status::UpdateElectionVotingStatusInput>,
    claims: JwtClaims,
    services: &State<HarvestServices>,
) -> Result<Json<UpdateElectionVotingStatusOutput>, Guarded<(Status, String)>> {
    // Check if the user has the required "Gold" role
    if !has_gold_permission(&claims) {
        return Err(
            (Status::Forbidden, "Insufficient privileges".to_string()).into()
        );
    }

    authorize(
        &claims,
        true,
        Some(claims.hasura_claims.tenant_id.clone()),
        vec![Permissions::ELECTION_STATE_WRITE],
    )?;
    let input = body.into_inner();
    let tenant_id = claims.hasura_claims.tenant_id.clone();
    let user_id = claims.hasura_claims.user_id.clone();
    let username = claims.preferred_username.clone();

    let mut hasura_db_client: DbClient =
        services.databases.hasura().await.get().await.map_err(|e| {
            (
                Status::InternalServerError,
                format!("Error getting hasura client {:?}", e),
            )
        })?;

    let hasura_transaction: deadpool_postgres::Transaction = hasura_db_client
        .transaction()
        .await
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;

    // Opening and closing a Post may need signatures first.
    let outcome = gate_election_status(
        &hasura_transaction,
        &caller(&claims),
        &tenant_id,
        &input.election_event_id,
        &input.election_id,
        &input.voting_status,
        &input.voting_channels,
    )
    .await?;
    if let Some(signing_request) = waiting(outcome) {
        hasura_transaction
            .commit()
            .await
            .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;
        kick_signing_log_outbox();
        return Ok(Json(UpdateElectionVotingStatusOutput {
            election_id: input.election_id,
            signing_request: Some(signing_request),
        }));
    }

    voting_status::update_election_status(
        tenant_id,
        Some(&user_id),
        username.as_deref(),
        &hasura_transaction,
        &input.election_event_id,
        &input.election_id,
        &input.voting_status,
        &input.voting_channels,
    )
    .await
    .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;

    let _commit = hasura_transaction
        .commit()
        .await
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;

    Ok(Json(UpdateElectionVotingStatusOutput {
        election_id: input.election_id.clone(),
        signing_request: None,
    }))
}
