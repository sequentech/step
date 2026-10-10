// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::services::authorization::{
    authorize, authorize_election_permission_labels,
};
use crate::services::dependencies::HarvestServices;
use crate::services::signing_gate::{
    caller, signing_required, waiting, Guarded,
};
use crate::types::error_response::{ErrorCode, ErrorResponse, JsonError};
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
use windmill::tasks::seal_ballot_boxes::kick_ballot_box_sealer;
use windmill::tasks::signing_log_outbox::kick_signing_log_outbox;

fn voting_response_error((status, message): (Status, String)) -> JsonError {
    let code = if status == Status::BadRequest {
        ErrorCode::VotingStatusValidation
    } else if status == Status::Forbidden || status == Status::Unauthorized {
        ErrorCode::Unauthorized
    } else {
        tracing::error!("Voting status request failed: {message}");
        return ErrorResponse::new(
            status,
            "Could not update voting status.",
            ErrorCode::InternalServerError,
        );
    };
    ErrorResponse::new(status, &message, code)
}

fn voting_service_error(error: anyhow::Error) -> (Status, String) {
    if let Some(refusal) =
        error.downcast_ref::<election_event_status::VotingTransitionError>()
    {
        (Status::BadRequest, refusal.to_string())
    } else {
        tracing::error!("Voting status operation failed: {error:?}");
        (
            Status::InternalServerError,
            "Could not update voting status.".into(),
        )
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct UpdateEventVotingStatusInput {
    pub election_event_id: String,
    pub voting_status: VotingStatus,
    pub voting_channels: Option<Vec<VotingStatusChannel>>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct UpdateEventVotingStatusOutput {
    pub election_event_id: String,
    /// The Posts the change left as they were, and why (e.g. with Seal at
    /// close, a Start leaves closed Posts closed). Left out when empty.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub skipped_elections: Vec<election_event_status::SkippedElection>,
}

#[instrument(skip(claims, services))]
#[post("/update-event-voting-status", format = "json", data = "<body>")]
pub async fn update_event_status(
    body: Json<UpdateEventVotingStatusInput>,
    claims: JwtClaims,
    services: &State<HarvestServices>,
) -> Result<Json<UpdateEventVotingStatusOutput>, Guarded<JsonError>> {
    update_event_status_response(body, claims, services)
        .await
        .map_err(|error| error.map_route(voting_response_error))
}

async fn update_event_status_response(
    body: Json<UpdateEventVotingStatusInput>,
    claims: JwtClaims,
    services: &HarvestServices,
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
    let user_id = claims.hasura_claims.user_id.clone();
    let username = claims.preferred_username.clone();

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

    authorize_election_permission_labels(
        &hasura_transaction,
        &claims,
        &input.election_event_id,
        None,
    )
    .await?;

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

    let (_, skipped_elections) =
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
        .map_err(voting_service_error)?;

    let _commit = hasura_transaction
        .commit()
        .await
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;
    // The close may have created ballot box seals to make (VOTE-FREEZE).
    if input.voting_status == VotingStatus::CLOSED {
        kick_ballot_box_sealer();
    }

    Ok(Json(UpdateEventVotingStatusOutput {
        election_event_id: input.election_event_id.clone(),
        skipped_elections,
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
) -> Result<Json<UpdateElectionVotingStatusOutput>, Guarded<JsonError>> {
    update_election_status_response(body, claims, services)
        .await
        .map_err(|error| error.map_route(voting_response_error))
}

async fn update_election_status_response(
    body: Json<voting_status::UpdateElectionVotingStatusInput>,
    claims: JwtClaims,
    services: &HarvestServices,
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

    authorize_election_permission_labels(
        &hasura_transaction,
        &claims,
        &input.election_event_id,
        Some(std::slice::from_ref(&input.election_id)),
    )
    .await?;

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
    .map_err(voting_service_error)?;

    let _commit = hasura_transaction
        .commit()
        .await
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;
    // The close may have created ballot box seals to make (VOTE-FREEZE).
    if input.voting_status == VotingStatus::CLOSED {
        kick_ballot_box_sealer();
    }

    Ok(Json(UpdateElectionVotingStatusOutput {
        election_id: input.election_id.clone(),
        signing_request: None,
    }))
}
