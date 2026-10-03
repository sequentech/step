// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! The cast ballot route with a scripted ballot box: who it casts for, the
//! receipt it answers with and the answer each refusal maps to.

use crate::adapters::memory::cast_votes::{Attempt, ScriptedCastVotes};
use crate::route_services::{bearer, json, Services};
use crate::test_claims::Claims;
use rocket::http::{ContentType, Header, Status};
use rocket::local::asynchronous::{Client, LocalResponse};
use sequent_core::ballot::VotingStatusChannel;
use sequent_core::ballot_receipt::{CastReceipt, CastReceiptStatement};
use sequent_core::types::permissions::VoterPermissions;
use serde_json::{json, Value};
use windmill::services::cast_ballot::{CastBallotOutput, CastBallotResult};
use windmill::services::cast_votes::{CastVote, CastVoteStatus};
use windmill::services::insert_cast_vote::CastVoteError;

const TENANT_ID: &str = "00000000-0000-4000-8000-00000000000a";
const EVENT_ID: &str = "00000000-0000-4000-8000-00000000000e";
const ELECTION_ID: &str = "00000000-0000-4000-8000-000000000001";
const AREA_ID: &str = "00000000-0000-4000-8000-0000000000a1";
const CAST_VOTE_ID: &str = "00000000-0000-4000-8000-0000000000c1";
const VOTER_ID: &str = "voter";
const BALLOT_ID: &str = "FTBE-MHRX";

fn voter() -> Claims {
    Claims::new(TENANT_ID, VOTER_ID)
        .azp("voting-portal")
        .username("voter.one")
        .auth_time(1727270400)
        .area(AREA_ID)
        .election_event(EVENT_ID)
        .authorized_elections(&[ELECTION_ID])
        .roles([VoterPermissions::CAST_VOTE])
}

fn receipt() -> CastBallotOutput {
    CastBallotOutput {
        receipt: CastReceipt {
            statement: CastReceiptStatement {
                election_event_id: EVENT_ID.into(),
                election_id: ELECTION_ID.into(),
                ballot_id: BALLOT_ID.into(),
                received_at: "2028-05-08T03:00:00.000Z".into(),
                cast_at: "2028-05-08T03:04:05.678Z".into(),
                key_id: "fd110d301d2f077d".into(),
                cast_signature: "cast-signature".into(),
            },
            cast_receipt_signature: "ballot-box-signature".into(),
        },
        cast_vote_id: CAST_VOTE_ID.into(),
    }
}

fn pending_cast_vote() -> CastVote {
    CastVote {
        id: CAST_VOTE_ID.into(),
        tenant_id: TENANT_ID.into(),
        election_id: Some(ELECTION_ID.into()),
        area_id: Some(AREA_ID.into()),
        created_at: None,
        last_updated_at: None,
        content: Some("ballot".into()),
        voter_id_string: Some(VOTER_ID.into()),
        election_event_id: EVENT_ID.into(),
        ballot_id: Some(BALLOT_ID.into()),
        cast_ballot_signature: None,
        status: CastVoteStatus::InProgress,
    }
}

async fn cast<'c>(client: &'c Client, claims: &Claims) -> LocalResponse<'c> {
    client
        .post("/cast-ballot")
        .header(ContentType::JSON)
        .header(bearer(claims))
        .header(Header::new("CF-Connecting-IP", "192.0.2.10"))
        .header(Header::new("CF-IPCountry", "ES"))
        .body(
            json!({
                "ballot_id": BALLOT_ID,
                "election_id": ELECTION_ID,
                "cast_signature": "cast-signature"
            })
            .to_string(),
        )
        .dispatch()
        .await
}

fn attempt() -> Attempt {
    Attempt {
        ballot_id: BALLOT_ID.into(),
        election_id: ELECTION_ID.into(),
        content: "cast-signature".into(),
        tenant_id: TENANT_ID.into(),
        auth_time: Some(1727270400),
        username: Some("voter.one".into()),
        voter_id: VOTER_ID.into(),
        area_id: AREA_ID.into(),
        voting_channel: VotingStatusChannel::ONLINE,
        voter_ip: Some("192.0.2.10".into()),
        voter_country: Some("ES".into()),
    }
}

#[rocket::async_test]
async fn a_cast_is_answered_with_the_ballot_boxs_receipt() {
    let services = Services::without_database().with_cast_votes(
        ScriptedCastVotes::casting([Ok(CastBallotResult::Cast(receipt()))]),
    );
    let client = services.client().await;

    let (status, body) = json(cast(&client, &voter()).await).await;
    assert_eq!(status, Status::Ok);
    assert_eq!(
        body,
        json!({
            "election_event_id": EVENT_ID,
            "election_id": ELECTION_ID,
            "ballot_id": BALLOT_ID,
            "received_at": "2028-05-08T03:00:00.000Z",
            "cast_at": "2028-05-08T03:04:05.678Z",
            "key_id": "fd110d301d2f077d",
            "cast_signature": "cast-signature",
            "cast_receipt_signature": "ballot-box-signature",
            "cast_vote_id": CAST_VOTE_ID,
        })
    );
    assert_eq!(services.cast_votes.cast_attempts(), vec![attempt()]);
    assert!(services.cast_votes.attempts().is_empty());
    assert!(services.tasks.sent().is_empty());
}

#[rocket::async_test]
async fn a_cast_pending_datafix_is_sent_for_processing_with_its_receipt() {
    let services = Services::without_database().with_cast_votes(
        ScriptedCastVotes::casting([Ok(CastBallotResult::PendingDatafix(
            receipt(),
            pending_cast_vote(),
        ))]),
    );
    let client = services.client().await;

    let (status, body) = json(cast(&client, &voter()).await).await;
    assert_eq!(status, Status::Ok);
    assert_eq!(body["cast_receipt_signature"], "ballot-box-signature");
    let sent = services.tasks.sent();
    assert_eq!(sent.len(), 1);
    assert_eq!(sent[0].name, "process_cast_vote");
    assert_eq!(
        sent[0].kwargs,
        json!({
            "tenant_id": TENANT_ID,
            "election_event_id": EVENT_ID,
            "cast_vote_id": CAST_VOTE_ID,
        })
    );
}

#[rocket::async_test]
async fn a_voter_who_may_not_cast_in_the_election_casts_nothing() {
    let other_election = "00000000-0000-4000-8000-000000000002";
    for claims in [
        voter().roles(Vec::<VoterPermissions>::new()),
        voter().authorized_elections(&[other_election]),
    ] {
        let services = Services::without_database()
            .with_cast_votes(ScriptedCastVotes::casting([]));
        let client = services.client().await;

        let (status, body): (Status, Value) =
            json(cast(&client, &claims).await).await;
        assert_eq!(status, Status::Unauthorized);
        assert_eq!(body["extensions"]["code"], "Unauthorized");
        assert!(services.cast_votes.cast_attempts().is_empty());
    }
}

#[rocket::async_test]
async fn a_cast_the_ballot_box_refuses_is_a_client_error_and_is_not_retried() {
    use CastVoteError::*;
    for error in [
        BallotNotReceived,
        BallotCastSignatureFailed("bad signature".into()),
        BallotAlreadyCast,
        BallotAudited,
    ] {
        let name = error.to_string();
        let services = Services::without_database().with_cast_votes(
            ScriptedCastVotes::casting([Ok(
                CastBallotResult::SkipRetryFailure(error),
            )]),
        );
        let client = services.client().await;

        assert_eq!(
            json(cast(&client, &voter()).await).await,
            (
                Status::BadRequest,
                json!({
                    "message": "PokValidationFailed",
                    "extensions": {"code": "PokValidationFailed"}
                }),
            ),
            "{name}"
        );
        assert_eq!(services.cast_votes.cast_attempts().len(), 1, "{name}");
        assert!(services.tasks.sent().is_empty());
    }
}

#[rocket::async_test]
async fn a_failed_cast_is_retried_and_answers_with_the_receipt() {
    let services = Services::without_database().with_cast_votes(
        ScriptedCastVotes::casting([
            Err(CastVoteError::CommitFailed("connection reset".into())),
            Ok(CastBallotResult::Cast(receipt())),
        ]),
    );
    let client = services.client().await;

    let (status, body) = json(cast(&client, &voter()).await).await;
    assert_eq!(status, Status::Ok);
    assert_eq!(body["ballot_id"], BALLOT_ID);
    assert_eq!(services.cast_votes.cast_attempts().len(), 2);
}

#[rocket::async_test]
async fn a_cast_that_keeps_failing_answers_with_the_last_failure() {
    let services = Services::without_database().with_cast_votes(
        ScriptedCastVotes::casting((0..6).map(|_| {
            Err(CastVoteError::CommitFailed("connection reset".into()))
        })),
    );
    let client = services.client().await;

    assert_eq!(
        json(cast(&client, &voter()).await).await,
        (
            Status::InternalServerError,
            json!({
                "message": "InternalServerError",
                "extensions": {"code": "InternalServerError"}
            }),
        )
    );
    assert_eq!(services.cast_votes.cast_attempts().len(), 6);
}
