// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::ports::cast_votes::CastVoter;
use crate::routes::insert_cast_vote::cast_vote_error_response;
use crate::services::authorization::authorize_voter_election;
use crate::services::dependencies::HarvestServices;
use crate::types::error_response::{ErrorCode, ErrorResponse, JsonError};
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::State;
use sequent_core::services::connection::UserLocation;
use sequent_core::services::jwt::JwtClaims;
use sequent_core::types::permissions::VoterPermissions;
use std::time::Instant;
use tracing::{info, instrument};
use windmill::services::audit_ballot::{AuditBallotInput, AuditBallotOutput};

/// The voter audits a ballot the ballot box received at review. POST coming
/// from the frontend->Hasura->Harvest->Here.
///
/// The ballot box marks the received ballot audited, and records it, before
/// it answers; the voting portal shows the ballot's secret values only after
/// this answer. An audited ballot is never cast and a cast one is never
/// audited. Auditing the same ballot again answers the same.
#[instrument(skip_all)]
#[post("/audit-ballot", format = "json", data = "<body>")]
pub async fn audit_ballot(
    body: Json<AuditBallotInput>,
    claims: JwtClaims,
    user_info: UserLocation,
    services: &State<HarvestServices>,
) -> Result<Json<AuditBallotOutput>, JsonError> {
    let start = Instant::now();
    let input: AuditBallotInput = body.into_inner();
    let election_id = input.election_id.to_string();

    let (area_id, voting_channel) = authorize_voter_election(
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
    })?;

    let audited = services
        .cast_votes
        .try_audit(
            input,
            CastVoter {
                tenant_id: &claims.hasura_claims.tenant_id,
                voter_id: &claims.hasura_claims.user_id,
                area_id: &area_id,
                voting_channel,
                auth_time: &claims.auth_time,
                voter_ip: &user_info.ip.map(|ip| ip.to_string()),
                voter_country: &user_info
                    .country_code
                    .clone()
                    .map(|country_code| country_code.to_string()),
                username: &claims.preferred_username,
            },
        )
        .await
        .map_err(|audit_err| {
            let elapsed_ms = start.elapsed().as_millis();
            info!(
                "audit-ballot took {elapsed_ms} ms to complete but failed with error={audit_err:?}"
            );
            cast_vote_error_response(audit_err)
        })?;

    info!(
        phase = "request",
        duration_us = start.elapsed().as_micros() as u64,
        "audit-ballot route completed"
    );
    Ok(Json(audited))
}

#[cfg(test)]
#[path = "../../tests/support/audit_ballot_routes.rs"]
mod route_tests;
