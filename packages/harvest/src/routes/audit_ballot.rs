// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::authorization::authorize_voter_election;
use crate::services::dependencies::HarvestServices;
use crate::services::vitrine::CastReceipts;
use electoral_log::adapters::ballot_box_received::AuditOutcome;
use rocket::{http::Status, serde::json::Json, State};
use sequent_core::ballot_receipt::normalize_ballot_id;
use sequent_core::services::jwt::JwtClaims;
use sequent_core::types::permissions::VoterPermissions;
use serde::Serialize;
use windmill::types::vitrine::VitrineCast;
use windmill::services::receive_ballot::{
    AuditBallotInput, AuditBallotOutput, AuditBallotStatus,
};

/// The audit acknowledgement, and what the event's Vitrine `ballots` log said
/// about the ballot's `audited` entry, or `None` when its transparency is off.
#[derive(Debug, Serialize)]
pub struct AuditBallotResponse {
    #[serde(flatten)]
    pub audit: AuditBallotOutput,
    pub vitrine: Option<VitrineCast>,
}

type ApiError = (Status, String);
fn unavailable(_: impl std::fmt::Display) -> ApiError {
    (Status::InternalServerError, "Unable to audit ballot".into())
}

/// Acknowledgement is sent only after audit wins the atomic race with cast.
/// The browser must obtain it before exposing the auditable plaintext.
#[tracing::instrument(skip_all)]
#[post("/audit-ballot", format = "json", data = "<body>")]
pub async fn audit_ballot(
    body: Json<AuditBallotInput>,
    claims: JwtClaims,
    services: &State<HarvestServices>,
    receipts: &State<CastReceipts>,
) -> Result<Json<AuditBallotResponse>, ApiError> {
    let mut input = body.into_inner();
    let (area, _) = authorize_voter_election(
        &claims,
        vec![VoterPermissions::CAST_VOTE],
        &input.election_id.to_string(),
    )?;
    let event_id = claims
        .hasura_claims
        .election_event_id
        .as_deref()
        .ok_or_else(|| {
            (Status::Forbidden, "A voter event is required".into())
        })?;
    input.ballot_id = normalize_ballot_id(&input.ballot_id)
        .ok_or_else(|| (Status::BadRequest, "Invalid Ballot ID".into()))?;
    let mut client = services
        .databases
        .hasura()
        .await
        .get()
        .await
        .map_err(unavailable)?;
    let mut tx = client.transaction().await.map_err(unavailable)?;
    let event =
        windmill::postgres::election_event::get_election_event_by_id_if_exist(
            &tx,
            &claims.hasura_claims.tenant_id,
            event_id,
        )
        .await
        .map_err(unavailable)?
        .ok_or_else(|| (Status::NotFound, "Ballot not found".into()))?;
    let store =
        windmill::services::electoral_log_board::get_event_store(event_id)
            .await
            .map_err(unavailable)?;
    let (outcome, vitrine) = windmill::services::receive_ballot::audit_ballot_for_voter(
        &store,
        &mut tx,
        &claims.hasura_claims.tenant_id,
        &event,
        &area,
        &claims.hasura_claims.user_id,
        &input,
    )
    .await
    .map_err(unavailable)?;
    match outcome {
        AuditOutcome::Audited => {
            let vitrine = receipts.cast(vitrine).await;
            Ok(Json(AuditBallotResponse {
                audit: AuditBallotOutput {
                    ballot_id: input.ballot_id,
                    status: AuditBallotStatus::Audited,
                },
                vitrine,
            }))
        }
        AuditOutcome::AlreadyCast => {
            Err((Status::Conflict, "A cast ballot cannot be audited".into()))
        }
        AuditOutcome::NotFound => {
            Err((Status::NotFound, "Ballot not found".into()))
        }
    }
}

#[cfg(test)]
#[path = "../../tests/support/audit_ballot_routes.rs"]
mod route_tests;
