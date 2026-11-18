// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::routes::ballot_checks::{ensure_checks_open, voter_check_scope};
use anyhow::Result;
use rocket::http::Status;
use rocket::serde::json::Json;
use sequent_core::ballot_receipt::normalize_ballot_id;
use sequent_core::services::jwt::JwtClaims;
use sequent_core::services::uuid_validation::parse_uuid_v4_field;
use sequent_core::types::hasura::core::TasksExecution;
use serde::{Deserialize, Serialize};
use tracing::{event, instrument, Level};
use windmill::services::ballot_receipt_tasks::{
    claim_ballot_receipt_task, BallotReceiptKey,
};
use windmill::services::celery_app::try_get_celery_app;
use windmill::services::reports::ballot_receipt::voter_cast_ballot;
use windmill::services::tasks_execution::update_fail;

#[derive(Serialize, Deserialize, Debug)]
pub struct CreateBallotReceiptInput {
    pub ballot_id: String,
    pub ballot_tracker_url: String,
    pub election_event_id: String,
    pub election_id: String,
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

/// Returns 400 for malformed ids, then checks the voter's own election event
/// and the requested election.
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
    let area_id = voter_check_scope(claims, &election_event_id, &election_id)?;
    Ok(ReceiptScope {
        area_id,
        election_event_id,
        election_id,
    })
}

fn unavailable() -> (Status, String) {
    (
        Status::InternalServerError,
        "Receipt unavailable".to_string(),
    )
}

/// Queues a ballot receipt for a ballot the voter cast in an election of
/// their own election event. A receipt is rendered once per ballot: repeated
/// requests get the render in progress or the finished document.
#[instrument(skip_all)]
#[post("/create-ballot-receipt", format = "json", data = "<body>")]
pub async fn create_ballot_receipt(
    body: Json<CreateBallotReceiptInput>,
    claims: JwtClaims,
) -> Result<Json<CreateBallotReceiptOutput>, (Status, String)> {
    let input = body.into_inner();
    let tenant_id = claims.hasura_claims.tenant_id.clone();
    let executer_name = String::new();

    let scope = authorize_receipt(&claims, &input)?;
    ensure_checks_open(&tenant_id, &scope.election_event_id).await?;
    let celery_app = try_get_celery_app().await.map_err(|error| {
        (
            Status::InternalServerError,
            format!("The task queues are not available: {error:#}"),
        )
    })?;

    let mut client = windmill::services::database::get_hasura_pool()
        .await
        .get()
        .await
        .map_err(|_| unavailable())?;
    let tx = client.transaction().await.map_err(|_| unavailable())?;
    // Resolve before enqueueing: a durable task must not pair a real voter
    // identifier with the ballot or its receipt document.
    let event = windmill::postgres::election_event::get_election_event_by_id(
        &tx,
        &tenant_id,
        &scope.election_event_id,
    )
    .await
    .map_err(|_| unavailable())?;
    let voter_id = windmill::services::vote_privacy::ballot_identity(
        &event,
        &claims.hasura_claims.user_id,
    )
    .await
    .map_err(|_| (Status::Forbidden, "Receipt unavailable".to_string()))?;
    let ballot_cast = voter_cast_ballot(
        &tx,
        &tenant_id,
        &scope.election_event_id,
        &scope.election_id,
        &voter_id,
        &scope.area_id,
        &input.ballot_id,
    )
    .await
    .map_err(|_| unavailable())?;
    if !ballot_cast {
        return Err((Status::NotFound, "Receipt unavailable".to_string()));
    }
    // Spellings of one receipt Ballot ID are one receipt.
    let receipt_ballot_id = normalize_ballot_id(&input.ballot_id)
        .unwrap_or_else(|| input.ballot_id.clone());
    let claim = claim_ballot_receipt_task(
        &tx,
        &BallotReceiptKey {
            tenant_id: &tenant_id,
            election_event_id: &scope.election_event_id,
            election_id: &scope.election_id,
            voter_id: &voter_id,
            ballot_id: &receipt_ballot_id,
            is_kiosk: false,
        },
        &executer_name,
    )
    .await
    .map_err(|_| unavailable())?;
    tx.commit().await.map_err(|_| unavailable())?;

    if claim.queue_render {
        let queued = async {
            windmill::services::electoral_log_board::get_event_store(
                &scope.election_event_id,
            )
            .await?
            .register_receipt_document(
                &scope.election_event_id,
                &claim.document_id,
                &voter_id,
            )
            .await?;
            celery_app
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
                        claim.task_execution.clone(),
                    ),
                )
                .await?;
            anyhow::Ok(())
        }
        .await;
        if let Err(error) = queued {
            // Let the next request claim the receipt again.
            if let Err(fail_error) =
                update_fail(&claim.task_execution, "Receipt not queued").await
            {
                event!(
                    Level::ERROR,
                    "Error marking the receipt task as failed: {fail_error:?}"
                );
            }
            event!(Level::ERROR, "Error queueing the receipt: {error:?}");
            return Err(unavailable());
        }
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
    use crate::test_claims::Claims;

    const EVENT: &str = "b7e2d915-3c4a-4e8f-9a61-5d0f2c8b7e43";
    const OTHER_EVENT: &str = "4f1c6b38-9f5e-4a59-8d3b-2a7f0c1e5d61";
    const ELECTION: &str = "0d3f6e2a-8b1c-4f7e-9a5d-6c2b1e0f4a87";

    fn voter() -> JwtClaims {
        Claims::new("tenant", "voter")
            .azp("voting-portal")
            .area("area")
            .election_event(EVENT)
            .authorized_elections(&[ELECTION])
            .roles(["user"])
            .build()
    }

    fn request(
        election_event_id: &str,
        election_id: &str,
    ) -> CreateBallotReceiptInput {
        CreateBallotReceiptInput {
            ballot_id: "ballot".into(),
            ballot_tracker_url: "https://example.com/ballot".into(),
            election_event_id: election_event_id.into(),
            election_id: election_id.into(),
        }
    }

    /// Another election event is refused; the voter's own is accepted.
    #[test]
    fn receipt_requires_the_voters_own_election_event() {
        assert_eq!(
            authorize_receipt(&voter(), &request(OTHER_EVENT, ELECTION))
                .unwrap_err()
                .0,
            Status::Forbidden
        );
        assert_eq!(
            authorize_receipt(&voter(), &request(EVENT, ELECTION)).unwrap(),
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
            &voter(),
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
                authorize_receipt(&voter(), &input).unwrap_err().0,
                Status::BadRequest
            );
        }
    }
}
