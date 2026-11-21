// SPDX-FileCopyrightText: 2024 Kevin Nguyen <kevin@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::authorization::{
    authorize_voter_election, ensure_election_in_event,
};
use anyhow::Result;
use deadpool_postgres::Client as DbClient;
use rocket::http::Status;
use rocket::serde::json::Json;
use sequent_core::services::uuid_validation::parse_uuid_v4_field;
use sequent_core::types::date_time::DateFormat;
use sequent_core::types::hasura::core::TasksExecution;
use sequent_core::types::permissions::VoterPermissions;
use sequent_core::{services::jwt::JwtClaims, types::date_time::TimeZone};
use serde::{Deserialize, Serialize};
use tracing::{event, instrument, Level};
use windmill::services::ballot_receipt_tasks::{
    claim_ballot_receipt_task, BallotReceiptKey,
};
use windmill::services::celery_app::get_celery_app;
use windmill::services::database::get_hasura_pool;
use windmill::services::reports::ballot_receipt::voter_cast_ballot;
use windmill::services::tasks_execution::update_fail;

#[derive(Serialize, Deserialize, Debug)]
pub struct CreateBallotReceiptInput {
    pub ballot_id: String,
    pub ballot_tracker_url: String,
    pub election_event_id: String,
    pub election_id: String,
    pub time_zone: Option<TimeZone>,
    pub date_format: Option<DateFormat>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CreateBallotReceiptOutput {
    pub id: String,
    pub ballot_id: String,
    pub status: String,
    pub task_execution: TasksExecution,
}

/// The voter's area and the requested ids in canonical (lower-case) form.
#[derive(Debug, PartialEq)]
struct ReceiptScope {
    area_id: String,
    election_event_id: String,
    election_id: String,
}

/// Returns 400 for malformed ids, then checks CAST_VOTE for the requested
/// election. The election event is bound to the election by
/// `ensure_election_in_event`.
fn authorize_receipt(
    claims: &JwtClaims,
    input: &CreateBallotReceiptInput,
) -> Result<ReceiptScope, (Status, String)> {
    let canonical = |value: &str, field: &str| {
        parse_uuid_v4_field(value, field)
            .map(|uuid| uuid.to_string())
            .map_err(|error| (Status::BadRequest, error.to_string()))
    };
    let election_event_id =
        canonical(&input.election_event_id, "election_event_id")?;
    let election_id = canonical(&input.election_id, "election_id")?;
    let (area_id, _) = authorize_voter_election(
        claims,
        vec![VoterPermissions::CAST_VOTE],
        &election_id,
    )?;
    Ok(ReceiptScope {
        area_id,
        election_event_id,
        election_id,
    })
}

/// Queues a ballot receipt for a ballot the voter cast in an election of
/// the requested election event. A receipt is rendered once per ballot: repeated
/// requests get the render in progress or the finished document.
#[instrument(skip_all)]
#[post("/create-ballot-receipt", format = "json", data = "<body>")]
pub async fn create_ballot_receipt(
    body: Json<CreateBallotReceiptInput>,
    claims: JwtClaims,
) -> Result<Json<CreateBallotReceiptOutput>, (Status, String)> {
    let input = body.into_inner();
    let tenant_id = claims.hasura_claims.tenant_id.clone();
    let executer_name = claims
        .name
        .clone()
        .unwrap_or_else(|| claims.hasura_claims.user_id.clone());

    let scope = authorize_receipt(&claims, &input)?;
    ensure_election_in_event(
        &tenant_id,
        &scope.election_event_id,
        &scope.election_id,
    )
    .await?;

    // The receipt link follows the voter's token client, not the request's
    // tracker URL, which Windmill ignores.
    let is_kiosk = claims.azp == "voting-portal-kiosk";
    let voter_id = claims.hasura_claims.user_id.clone();
    let internal_error = |context: &str, error: anyhow::Error| {
        (Status::InternalServerError, format!("{context}: {error:?}"))
    };

    let mut hasura_db_client: DbClient =
        get_hasura_pool().await.get().await.map_err(|error| {
            internal_error("Error obtaining hasura client", error.into())
        })?;
    let hasura_transaction =
        hasura_db_client.transaction().await.map_err(|error| {
            internal_error("Error obtaining hasura transaction", error.into())
        })?;
    let ballot_cast = voter_cast_ballot(
        &hasura_transaction,
        &tenant_id,
        &scope.election_event_id,
        &scope.election_id,
        &voter_id,
        &scope.area_id,
        &input.ballot_id,
    )
    .await
    .map_err(|error| internal_error("Error checking the cast ballot", error))?;
    if !ballot_cast {
        return Err((Status::NotFound, "Ballot not found".to_string()));
    }
    let claim = claim_ballot_receipt_task(
        &hasura_transaction,
        &BallotReceiptKey {
            tenant_id: &tenant_id,
            election_event_id: &scope.election_event_id,
            election_id: &scope.election_id,
            voter_id: &voter_id,
            ballot_id: &input.ballot_id,
            is_kiosk,
        },
        &executer_name,
    )
    .await
    .map_err(|error| {
        internal_error("Error recording the receipt task", error)
    })?;
    hasura_transaction.commit().await.map_err(|error| {
        internal_error("Error committing the receipt task", error.into())
    })?;

    if claim.queue_render {
        let celery_app = get_celery_app().await;
        let celery_task_result = celery_app
            .send_task(
                windmill::tasks::create_ballot_receipt::create_ballot_receipt::new(
                    claim.document_id.clone(),
                    input.ballot_id.clone(),
                    input.ballot_tracker_url,
                    tenant_id.clone(),
                    scope.election_event_id,
                    scope.election_id,
                    scope.area_id,
                    voter_id,
                    input.time_zone,
                    input.date_format,
                    claim.task_execution.clone(),
                    Some(is_kiosk),
                ),
            )
            .await;
        match celery_task_result {
            Ok(task) => info!("Sent task {:?} successfully", task),
            Err(error) => {
                // Let the next request claim the receipt again.
                let message = format!(
                    "Error sending create_ballot_receipt task: {error:?}"
                );
                if let Err(fail_error) =
                    update_fail(&claim.task_execution, &message).await
                {
                    event!(
                        Level::ERROR,
                        "Error marking the receipt task as failed: {fail_error:?}"
                    );
                }
                return Err((Status::InternalServerError, message));
            }
        }
    } else {
        info!(
            "Reusing receipt task {} for ballot {}",
            claim.task_execution.id, input.ballot_id
        );
    }

    Ok(Json(CreateBallotReceiptOutput {
        id: claim.document_id,
        ballot_id: input.ballot_id,
        status: "pending".to_string(),
        task_execution: claim.task_execution,
    }))
}

#[cfg(test)]
mod create_ballot_receipt_scope_tests {
    use super::*;

    const EVENT: &str = "b7e2d915-3c4a-4e8f-9a61-5d0f2c8b7e43";
    const ELECTION: &str = "0d3f6e2a-8b1c-4f7e-9a5d-6c2b1e0f4a87";
    const OTHER_ELECTION: &str = "6a9e4c21-7d3b-4b8f-8e15-2f0c9d7a3b64";

    /// Voting portal claims authorized for the given elections.
    fn voter(election_ids: &[&str]) -> JwtClaims {
        serde_json::from_value(serde_json::json!({
            "exp": 1, "iat": 0, "jti": "test", "iss": "test",
            "sub": "voter", "typ": "Bearer", "azp": "voting-portal",
            "acr": "1", "allowed-origins": [], "scope": "openid",
            "email_verified": false,
            "https://hasura.io/jwt/claims": {
                "x-hasura-default-role": "user",
                "x-hasura-tenant-id": "tenant",
                "x-hasura-user-id": "voter",
                "x-hasura-area-id": "area",
                "authorized-election-ids": election_ids,
                "x-hasura-allowed-roles": [VoterPermissions::CAST_VOTE.to_string()]
            }
        }))
        .expect("valid test claims")
    }

    /// Receipt request for the given election event and election.
    fn request(
        election_event_id: &str,
        election_id: &str,
    ) -> CreateBallotReceiptInput {
        CreateBallotReceiptInput {
            ballot_id: "ballot".into(),
            ballot_tracker_url: "https://example.com/ballot".into(),
            election_event_id: election_event_id.into(),
            election_id: election_id.into(),
            time_zone: None,
            date_format: None,
        }
    }

    /// The election must be one of the token's authorized elections.
    #[test]
    fn receipt_requires_an_authorized_election() {
        assert_eq!(
            authorize_receipt(
                &voter(&[OTHER_ELECTION]),
                &request(EVENT, ELECTION)
            )
            .unwrap_err()
            .0,
            Status::Unauthorized
        );
        assert_eq!(
            authorize_receipt(&voter(&[ELECTION]), &request(EVENT, ELECTION))
                .unwrap(),
            ReceiptScope {
                area_id: "area".into(),
                election_event_id: EVENT.into(),
                election_id: ELECTION.into(),
            }
        );
    }

    /// Ids are compared as UUIDs, so letter case does not matter, and the
    /// route continues with the canonical ids.
    #[test]
    fn receipt_ids_are_compared_as_uuids() {
        let scope = authorize_receipt(
            &voter(&[ELECTION]),
            &request(&EVENT.to_uppercase(), &ELECTION.to_uppercase()),
        )
        .unwrap();
        assert_eq!(scope.election_event_id, EVENT);
        assert_eq!(scope.election_id, ELECTION);
    }

    /// Malformed ids are a client error, not an authorization failure.
    #[test]
    fn malformed_receipt_ids_get_400() {
        for input in [request("event", ELECTION), request(EVENT, "election")] {
            assert_eq!(
                authorize_receipt(&voter(&[ELECTION]), &input)
                    .unwrap_err()
                    .0,
                Status::BadRequest
            );
        }
    }
}
