// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! The receive ballot route with a scripted ballot box: who it receives for,
//! the receipt it answers with and the answer each refusal maps to.

use crate::adapters::memory::cast_votes::{Attempt, ScriptedCastVotes};
use crate::route_services::{bearer, json, Services};
use crate::test_claims::Claims;
use rocket::http::{ContentType, Header, Status};
use rocket::local::asynchronous::{Client, LocalResponse};
use sequent_core::ballot::VotingStatusChannel;
use sequent_core::ballot_receipt::{ReceivedBallot, ReceivedStatement};
use sequent_core::types::permissions::VoterPermissions;
use serde_json::{json, Value};
use windmill::services::insert_cast_vote::CastVoteError;

const TENANT_ID: &str = "00000000-0000-4000-8000-00000000000a";
const EVENT_ID: &str = "00000000-0000-4000-8000-00000000000e";
const ELECTION_ID: &str = "00000000-0000-4000-8000-000000000001";
const AREA_ID: &str = "00000000-0000-4000-8000-0000000000a1";
const VOTER_ID: &str = "voter";

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

fn received() -> ReceivedBallot {
    ReceivedBallot {
        statement: ReceivedStatement {
            tenant_id: TENANT_ID.into(),
            election_event_id: EVENT_ID.into(),
            election_id: ELECTION_ID.into(),
            ballot_hash: "ballot-hash".into(),
            voter_signing_pk: "voter-key".into(),
            voter_ballot_signature: "voter-signature".into(),
            received_at: "2028-05-08T03:00:00.000Z".into(),
            key_id: "fd110d301d2f077d".into(),
        },
        received_signature: "ballot-box-signature".into(),
        ballot_id: "FTBE-MHRX".into(),
    }
}

async fn receive<'c>(client: &'c Client, claims: &Claims) -> LocalResponse<'c> {
    client
        .post("/receive-ballot")
        .header(ContentType::JSON)
        .header(bearer(claims))
        .header(Header::new("CF-Connecting-IP", "192.0.2.10"))
        .header(Header::new("CF-IPCountry", "ES"))
        .body(
            json!({"ballot_id": "ballot-hash", "election_id": ELECTION_ID, "content": "ballot"})
                .to_string(),
        )
        .dispatch()
        .await
}

fn attempt() -> Attempt {
    Attempt {
        ballot_id: "ballot-hash".into(),
        election_id: ELECTION_ID.into(),
        content: "ballot".into(),
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
async fn a_received_ballot_is_answered_with_the_ballot_boxs_receipt() {
    let services = Services::without_database()
        .with_cast_votes(ScriptedCastVotes::receiving([Ok(received())]));
    let client = services.client().await;

    let (status, body) = json(receive(&client, &voter()).await).await;
    assert_eq!(status, Status::Ok);
    assert_eq!(
        body,
        json!({
            "tenant_id": TENANT_ID,
            "election_event_id": EVENT_ID,
            "election_id": ELECTION_ID,
            "ballot_hash": "ballot-hash",
            "voter_signing_pk": "voter-key",
            "voter_ballot_signature": "voter-signature",
            "received_at": "2028-05-08T03:00:00.000Z",
            "key_id": "fd110d301d2f077d",
            "received_signature": "ballot-box-signature",
            "ballot_id": "FTBE-MHRX",
        })
    );
    assert_eq!(services.cast_votes.receive_attempts(), vec![attempt()]);
    // Receiving is not casting.
    assert!(services.cast_votes.attempts().is_empty());
    assert!(services.tasks.sent().is_empty());
}

#[rocket::async_test]
async fn a_voter_who_may_not_cast_in_the_election_has_nothing_received() {
    let other_election = "00000000-0000-4000-8000-000000000002";
    for claims in [
        voter().roles(Vec::<VoterPermissions>::new()),
        voter().authorized_elections(&[other_election]),
    ] {
        let services = Services::without_database()
            .with_cast_votes(ScriptedCastVotes::receiving([]));
        let client = services.client().await;

        let (status, body): (Status, Value) =
            json(receive(&client, &claims).await).await;
        assert_eq!(status, Status::Unauthorized);
        assert_eq!(body["extensions"]["code"], "Unauthorized");
        assert!(services.cast_votes.receive_attempts().is_empty());
    }
}

#[rocket::async_test]
async fn a_ballot_the_ballot_box_refuses_is_a_client_error_and_is_not_retried()
{
    use CastVoteError::*;
    for error in [
        BallotVoterSignatureRequired,
        BallotVoterSignatureFailed("bad signature".into()),
        BallotStyleMismatch("another style".into()),
    ] {
        let name = error.to_string();
        let services = Services::without_database()
            .with_cast_votes(ScriptedCastVotes::receiving([Err(error)]));
        let client = services.client().await;

        assert_eq!(
            json(receive(&client, &voter()).await).await,
            (
                Status::BadRequest,
                json!({
                    "message": "PokValidationFailed",
                    "extensions": {"code": "PokValidationFailed"}
                }),
            ),
            "{name}"
        );
        assert_eq!(services.cast_votes.receive_attempts().len(), 1, "{name}");
    }
}

#[rocket::async_test]
async fn a_closed_election_and_a_storage_failure_keep_the_cast_answers() {
    for (error, status, message, code) in [
        (
            CastVoteError::CheckStatusFailed("closed".into()),
            Status::Unauthorized,
            "closed",
            "CheckStatusFailed",
        ),
        (
            CastVoteError::CommitFailed("connection reset".into()),
            Status::InternalServerError,
            "InternalServerError",
            "InternalServerError",
        ),
    ] {
        let services = Services::without_database()
            .with_cast_votes(ScriptedCastVotes::receiving([Err(error)]));
        let client = services.client().await;

        assert_eq!(
            json(receive(&client, &voter()).await).await,
            (
                status,
                json!({"message": message, "extensions": {"code": code}})
            ),
            "{code}"
        );
    }
}
